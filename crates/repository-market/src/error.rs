//! Error classification and retry strategy for market data repository.

use std::fmt;
use std::time::Duration;
use tracing::{debug, warn};

/// Error classification for storage operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// Transient error: network timeout, temporary unavailability.
    /// Safe to retry with backoff.
    Transient,
    /// Permanent error: invalid parameters, authentication failure.
    /// Do not retry.
    Permanent,
    /// Data corruption: manifest/file inconsistency, invalid Parquet schema.
    /// Log alert, degrade gracefully, do not retry.
    DataCorruption,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::Transient => write!(f, "transient"),
            ErrorKind::Permanent => write!(f, "permanent"),
            ErrorKind::DataCorruption => write!(f, "data_corruption"),
        }
    }
}

/// Classify an error into an ErrorKind based on error message heuristics.
///
/// This is a best-effort classification. Callers with more context
/// should override when possible.
pub fn classify_error(err: &anyhow::Error) -> ErrorKind {
    let msg = format!("{err:#}").to_lowercase();

    // Network / timeout patterns → transient
    if msg.contains("timeout")
        || msg.contains("timed out")
        || msg.contains("connection refused")
        || msg.contains("connection reset")
        || msg.contains("temporarily unavailable")
        || msg.contains("too many requests")
        || msg.contains("rate limit")
        || msg.contains("503")
        || msg.contains("429")
    {
        return ErrorKind::Transient;
    }

    // Corruption patterns
    if msg.contains("corrupt")
        || msg.contains("invalid schema")
        || msg.contains("invalid parquet")
        || msg.contains("not a valid parquet")
    {
        return ErrorKind::DataCorruption;
    }

    // Default: permanent (safe default — don't retry unknown errors)
    ErrorKind::Permanent
}

/// Retry configuration with exponential backoff.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts (0 = no retry).
    pub max_retries: u32,
    /// Base delay between retries in milliseconds.
    pub base_delay_ms: u64,
    /// Maximum delay cap in milliseconds.
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 200,
            max_delay_ms: 5_000,
        }
    }
}

impl RetryConfig {
    /// Calculate delay for a given attempt (0-indexed) with exponential backoff + jitter.
    fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let exp_delay = self.base_delay_ms.saturating_mul(1u64 << attempt.min(10));
        let capped = exp_delay.min(self.max_delay_ms);
        // Simple jitter: use attempt as seed for deterministic but varied delays
        let jitter = capped / 4;
        Duration::from_millis(capped.saturating_add(jitter))
    }
}

/// Retry an async operation, retrying only on transient errors.
///
/// Returns the first successful result, or the last error if all retries exhausted.
pub async fn retry_on_transient<F, Fut, T>(
    config: &RetryConfig,
    operation_name: &str,
    mut f: F,
) -> anyhow::Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<T>>,
{
    let mut last_err = None;

    for attempt in 0..=config.max_retries {
        match f().await {
            Ok(val) => {
                if attempt > 0 {
                    debug!(
                        operation = operation_name,
                        attempt,
                        "Retry succeeded"
                    );
                }
                return Ok(val);
            }
            Err(e) => {
                let kind = classify_error(&e);

                if kind != ErrorKind::Transient || attempt == config.max_retries {
                    if kind != ErrorKind::Transient {
                        debug!(
                            operation = operation_name,
                            error_kind = %kind,
                            error = %e,
                            "Non-transient error, not retrying"
                        );
                    } else {
                        warn!(
                            operation = operation_name,
                            attempt,
                            max_retries = config.max_retries,
                            error = %e,
                            "All retries exhausted"
                        );
                    }
                    return Err(e);
                }

                let delay = config.delay_for_attempt(attempt);
                warn!(
                    operation = operation_name,
                    attempt,
                    error_kind = %kind,
                    error = %e,
                    delay_ms = delay.as_millis() as u64,
                    "Transient error, retrying after delay"
                );

                last_err = Some(e);
                tokio::time::sleep(delay).await;
            }
        }
    }

    // Should not reach here, but handle gracefully
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("Retry loop completed without result")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn classify_timeout_as_transient() {
        let err = anyhow::anyhow!("connection timed out after 30s");
        assert_eq!(classify_error(&err), ErrorKind::Transient);
    }

    #[test]
    fn classify_rate_limit_as_transient() {
        let err = anyhow::anyhow!("HTTP 429 Too Many Requests - rate limit exceeded");
        assert_eq!(classify_error(&err), ErrorKind::Transient);
    }

    #[test]
    fn classify_connection_refused_as_transient() {
        let err = anyhow::anyhow!("connection refused");
        assert_eq!(classify_error(&err), ErrorKind::Transient);
    }

    #[test]
    fn classify_corrupt_as_data_corruption() {
        let err = anyhow::anyhow!("not a valid parquet file");
        assert_eq!(classify_error(&err), ErrorKind::DataCorruption);
    }

    #[test]
    fn classify_unknown_as_permanent() {
        let err = anyhow::anyhow!("invalid API key");
        assert_eq!(classify_error(&err), ErrorKind::Permanent);
    }

    #[test]
    fn retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.base_delay_ms, 200);
        assert_eq!(config.max_delay_ms, 5_000);
    }

    #[test]
    fn delay_capped_at_max() {
        let config = RetryConfig {
            max_retries: 10,
            base_delay_ms: 1000,
            max_delay_ms: 2000,
        };
        let delay = config.delay_for_attempt(5);
        // Should be capped at max_delay_ms + jitter
        assert!(delay.as_millis() <= 2500 + 1);
    }

    #[tokio::test]
    async fn retry_succeeds_after_transient_failure() {
        let call_count = Arc::new(Mutex::new(0u32));
        let cc = call_count.clone();

        let config = RetryConfig {
            max_retries: 3,
            base_delay_ms: 1, // minimal delay for test speed
            max_delay_ms: 5,
        };

        let result = retry_on_transient(&config, "test_op", || {
            let cc = cc.clone();
            async move {
                let mut count = cc.lock().unwrap();
                *count += 1;
                if *count < 3 {
                    anyhow::bail!("connection timed out"); // transient
                }
                Ok(42)
            }
        })
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(*call_count.lock().unwrap(), 3);
    }

    #[tokio::test]
    async fn retry_does_not_retry_permanent_error() {
        let call_count = Arc::new(Mutex::new(0u32));
        let cc = call_count.clone();

        let config = RetryConfig {
            max_retries: 3,
            base_delay_ms: 1,
            max_delay_ms: 5,
        };

        let result: anyhow::Result<i32> = retry_on_transient(&config, "test_op", || {
            let cc = cc.clone();
            async move {
                let mut count = cc.lock().unwrap();
                *count += 1;
                anyhow::bail!("invalid API key"); // permanent
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(*call_count.lock().unwrap(), 1, "Should not retry permanent errors");
    }

    #[tokio::test]
    async fn retry_exhausts_all_attempts_on_transient() {
        let call_count = Arc::new(Mutex::new(0u32));
        let cc = call_count.clone();

        let config = RetryConfig {
            max_retries: 2,
            base_delay_ms: 1,
            max_delay_ms: 5,
        };

        let result: anyhow::Result<i32> = retry_on_transient(&config, "test_op", || {
            let cc = cc.clone();
            async move {
                let mut count = cc.lock().unwrap();
                *count += 1;
                anyhow::bail!("connection timed out"); // always transient
            }
        })
        .await;

        assert!(result.is_err());
        // 1 initial + 2 retries = 3 total
        assert_eq!(*call_count.lock().unwrap(), 3, "Should exhaust all retries");
    }
}
