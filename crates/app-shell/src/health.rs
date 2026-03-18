//! Startup health checks and Windows runtime path initialization.
//!
//! Provides structured self-check on startup:
//! - Windows directory initialization and writability
//! - Database connectivity (optional, degrades gracefully)
//! - Disk space baseline check

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{info, warn, error};

/// Severity of a health check result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckSeverity {
    /// Informational — no action needed.
    Ok,
    /// Degraded — feature may be limited but app can continue.
    Warn,
    /// Blocking — critical subsystem unavailable.
    Error,
}

/// A single health check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub name: String,
    pub severity: CheckSeverity,
    pub message: String,
    pub checked_at: DateTime<Utc>,
}

/// Aggregated health report from startup self-check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub results: Vec<CheckResult>,
    pub created_at: DateTime<Utc>,
}

impl HealthReport {
    /// True if any check has Error severity.
    pub fn has_errors(&self) -> bool {
        self.results.iter().any(|r| r.severity == CheckSeverity::Error)
    }

    /// True if any check has Warn severity.
    pub fn has_warnings(&self) -> bool {
        self.results.iter().any(|r| r.severity == CheckSeverity::Warn)
    }

    /// Count of checks by severity.
    pub fn count_by_severity(&self, severity: CheckSeverity) -> usize {
        self.results.iter().filter(|r| r.severity == severity).count()
    }
}

// ---------------------------------------------------------------------------
// Windows path initialization
// ---------------------------------------------------------------------------

/// Standard Windows runtime directories for zquant.
#[derive(Debug, Clone)]
pub struct RuntimePaths {
    pub config_dir: PathBuf,
    pub log_dir: PathBuf,
    pub parquet_dir: PathBuf,
    pub tmp_dir: PathBuf,
}

/// Error classifying why path initialization failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathInitError {
    /// The required environment variable is not set.
    EnvVarMissing(String),
    /// Directory creation failed.
    CreateFailed(String),
    /// Directory exists but is not writable.
    NotWritable(String),
}

impl std::fmt::Display for PathInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathInitError::EnvVarMissing(var) => write!(f, "environment variable not set: {var}"),
            PathInitError::CreateFailed(msg) => write!(f, "directory creation failed: {msg}"),
            PathInitError::NotWritable(path) => write!(f, "directory not writable: {path}"),
        }
    }
}

/// Resolve the standard runtime paths from Windows environment variables.
/// Does NOT create directories — use `ensure_runtime_dirs` for that.
pub fn resolve_runtime_paths() -> Result<RuntimePaths, PathInitError> {
    let appdata = std::env::var("APPDATA")
        .map_err(|_| PathInitError::EnvVarMissing("APPDATA".into()))?;
    let local_appdata = std::env::var("LOCALAPPDATA")
        .map_err(|_| PathInitError::EnvVarMissing("LOCALAPPDATA".into()))?;

    Ok(RuntimePaths {
        config_dir: PathBuf::from(&appdata).join("zquant").join("config"),
        log_dir: PathBuf::from(&local_appdata).join("zquant").join("logs"),
        parquet_dir: PathBuf::from(&local_appdata).join("zquant").join("data").join("parquet"),
        tmp_dir: PathBuf::from(&local_appdata).join("zquant").join("tmp"),
    })
}

/// Create runtime directories and verify writability.
/// Returns a list of `CheckResult` for each directory.
pub fn ensure_runtime_dirs(paths: &RuntimePaths) -> Vec<CheckResult> {
    let dirs = [
        ("config_dir", &paths.config_dir),
        ("log_dir", &paths.log_dir),
        ("parquet_dir", &paths.parquet_dir),
        ("tmp_dir", &paths.tmp_dir),
    ];

    dirs.iter()
        .map(|(name, path)| ensure_single_dir(name, path))
        .collect()
}

fn ensure_single_dir(name: &str, path: &PathBuf) -> CheckResult {
    let now = Utc::now();

    // Create directory tree
    if let Err(e) = std::fs::create_dir_all(path) {
        return CheckResult {
            name: format!("dir:{name}"),
            severity: CheckSeverity::Error,
            message: format!("Cannot create {}: {e}", path.display()),
            checked_at: now,
        };
    }

    // Writability probe: create and remove a temp file
    let probe = path.join(".zquant_probe");
    match std::fs::write(&probe, b"ok") {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            CheckResult {
                name: format!("dir:{name}"),
                severity: CheckSeverity::Ok,
                message: format!("{} writable", path.display()),
                checked_at: now,
            }
        }
        Err(e) => CheckResult {
            name: format!("dir:{name}"),
            severity: CheckSeverity::Error,
            message: format!("{} not writable: {e}", path.display()),
            checked_at: now,
        },
    }
}

// ---------------------------------------------------------------------------
// Disk space check
// ---------------------------------------------------------------------------

/// Minimum free disk space threshold (500 MB).
const MIN_FREE_DISK_BYTES: u64 = 500 * 1024 * 1024;

/// Check free disk space on the drive containing `path`.
/// Uses a simple heuristic: read available space via std::fs metadata.
/// Falls back to Warn if the check itself fails (non-blocking).
pub fn check_disk_space(path: &PathBuf) -> CheckResult {
    let now = Utc::now();

    // On Windows, we use the `available_space` from fs2 or a manual approach.
    // For minimal dependency, we shell out or use a platform-specific call.
    // Here we use a safe fallback: try to get metadata, warn if unavailable.
    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        // Extract drive root (e.g., "C:\\")
        let drive_root = path
            .components()
            .next()
            .map(|c| {
                let mut p = PathBuf::from(c.as_os_str());
                p.push("\\");
                p
            })
            .unwrap_or_else(|| PathBuf::from("C:\\"));

        let wide: Vec<u16> = OsStr::new(&drive_root)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut free_bytes: u64 = 0;
        let mut total_bytes: u64 = 0;
        let mut total_free: u64 = 0;

        let ok = unsafe {
            GetDiskFreeSpaceExW(
                wide.as_ptr(),
                &mut free_bytes,
                &mut total_bytes,
                &mut total_free,
            )
        };

        if ok != 0 {
            let free_mb = free_bytes / (1024 * 1024);
            if free_bytes < MIN_FREE_DISK_BYTES {
                return CheckResult {
                    name: "disk:space".into(),
                    severity: CheckSeverity::Warn,
                    message: format!(
                        "Low disk space on {}: {free_mb} MB free (threshold: {} MB)",
                        drive_root.display(),
                        MIN_FREE_DISK_BYTES / (1024 * 1024)
                    ),
                    checked_at: now,
                };
            }
            return CheckResult {
                name: "disk:space".into(),
                severity: CheckSeverity::Ok,
                message: format!("{free_mb} MB free on {}", drive_root.display()),
                checked_at: now,
            };
        }
    }

    // Fallback for non-Windows or if the call failed
    CheckResult {
        name: "disk:space".into(),
        severity: CheckSeverity::Warn,
        message: "Could not determine free disk space".into(),
        checked_at: now,
    }
}

#[cfg(windows)]
extern "system" {
    fn GetDiskFreeSpaceExW(
        lpDirectoryName: *const u16,
        lpFreeBytesAvailableToCaller: *mut u64,
        lpTotalNumberOfBytes: *mut u64,
        lpTotalNumberOfFreeBytes: *mut u64,
    ) -> i32;
}

// ---------------------------------------------------------------------------
// Database connectivity check
// ---------------------------------------------------------------------------

/// Check database connectivity. Returns Ok severity on success, Warn on failure
/// (database is optional — app degrades to UI-only mode).
pub async fn check_database(database_url: &str) -> CheckResult {
    let now = Utc::now();

    match sqlx::PgPool::connect(database_url).await {
        Ok(pool) => {
            // Quick ping
            match sqlx::query("SELECT 1").execute(&pool).await {
                Ok(_) => {
                    pool.close().await;
                    CheckResult {
                        name: "db:connectivity".into(),
                        severity: CheckSeverity::Ok,
                        message: "Database connected".into(),
                        checked_at: now,
                    }
                }
                Err(e) => {
                    pool.close().await;
                    CheckResult {
                        name: "db:connectivity".into(),
                        severity: CheckSeverity::Warn,
                        message: format!("Database ping failed: {e}"),
                        checked_at: now,
                    }
                }
            }
        }
        Err(e) => CheckResult {
            name: "db:connectivity".into(),
            severity: CheckSeverity::Warn,
            message: format!("Database connection failed: {e}"),
            checked_at: now,
        },
    }
}

// ---------------------------------------------------------------------------
// Startup self-check orchestration
// ---------------------------------------------------------------------------

/// Run all startup health checks and return an aggregated report.
/// This is the main entry point called during application initialization.
pub async fn run_startup_checks(database_url: Option<&str>) -> HealthReport {
    info!("Running startup health checks");
    let mut results = Vec::new();

    // 1. Windows path initialization
    match resolve_runtime_paths() {
        Ok(paths) => {
            let dir_results = ensure_runtime_dirs(&paths);
            for r in &dir_results {
                match r.severity {
                    CheckSeverity::Ok => info!(check = %r.name, "{}", r.message),
                    CheckSeverity::Warn => warn!(check = %r.name, "{}", r.message),
                    CheckSeverity::Error => error!(check = %r.name, "{}", r.message),
                }
            }
            // Disk space check on the data directory's drive
            let disk = check_disk_space(&paths.parquet_dir);
            match disk.severity {
                CheckSeverity::Ok => info!(check = %disk.name, "{}", disk.message),
                CheckSeverity::Warn => warn!(check = %disk.name, "{}", disk.message),
                CheckSeverity::Error => error!(check = %disk.name, "{}", disk.message),
            }
            results.extend(dir_results);
            results.push(disk);
        }
        Err(e) => {
            error!("Runtime path resolution failed: {e}");
            results.push(CheckResult {
                name: "dir:resolve".into(),
                severity: CheckSeverity::Error,
                message: format!("Path resolution failed: {e}"),
                checked_at: Utc::now(),
            });
        }
    }

    // 2. Database connectivity (optional)
    if let Some(url) = database_url {
        let db_result = check_database(url).await;
        match db_result.severity {
            CheckSeverity::Ok => info!(check = %db_result.name, "{}", db_result.message),
            CheckSeverity::Warn => warn!(check = %db_result.name, "{}", db_result.message),
            CheckSeverity::Error => error!(check = %db_result.name, "{}", db_result.message),
        }
        results.push(db_result);
    } else {
        results.push(CheckResult {
            name: "db:connectivity".into(),
            severity: CheckSeverity::Warn,
            message: "No DATABASE_URL configured, running in UI-only mode".into(),
            checked_at: Utc::now(),
        });
    }

    let report = HealthReport {
        created_at: Utc::now(),
        results,
    };

    let ok_count = report.count_by_severity(CheckSeverity::Ok);
    let warn_count = report.count_by_severity(CheckSeverity::Warn);
    let err_count = report.count_by_severity(CheckSeverity::Error);
    info!(
        ok = ok_count,
        warn = warn_count,
        error = err_count,
        "Startup health check complete"
    );

    report
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn health_report_severity_counts() {
        let report = HealthReport {
            created_at: Utc::now(),
            results: vec![
                CheckResult {
                    name: "a".into(),
                    severity: CheckSeverity::Ok,
                    message: "ok".into(),
                    checked_at: Utc::now(),
                },
                CheckResult {
                    name: "b".into(),
                    severity: CheckSeverity::Warn,
                    message: "warn".into(),
                    checked_at: Utc::now(),
                },
                CheckResult {
                    name: "c".into(),
                    severity: CheckSeverity::Error,
                    message: "err".into(),
                    checked_at: Utc::now(),
                },
            ],
        };
        assert_eq!(report.count_by_severity(CheckSeverity::Ok), 1);
        assert_eq!(report.count_by_severity(CheckSeverity::Warn), 1);
        assert_eq!(report.count_by_severity(CheckSeverity::Error), 1);
        assert!(report.has_errors());
        assert!(report.has_warnings());
    }

    #[test]
    fn health_report_no_errors() {
        let report = HealthReport {
            created_at: Utc::now(),
            results: vec![CheckResult {
                name: "a".into(),
                severity: CheckSeverity::Ok,
                message: "ok".into(),
                checked_at: Utc::now(),
            }],
        };
        assert!(!report.has_errors());
        assert!(!report.has_warnings());
    }

    #[test]
    fn resolve_runtime_paths_uses_env_vars() {
        // Set env vars for test
        std::env::set_var("APPDATA", "C:\\Users\\test\\AppData\\Roaming");
        std::env::set_var("LOCALAPPDATA", "C:\\Users\\test\\AppData\\Local");

        let paths = resolve_runtime_paths().expect("should resolve");
        assert!(paths.config_dir.ends_with("zquant\\config"));
        assert!(paths.log_dir.ends_with("zquant\\logs"));
        assert!(paths.parquet_dir.ends_with("zquant\\data\\parquet"));
        assert!(paths.tmp_dir.ends_with("zquant\\tmp"));
    }

    #[test]
    fn ensure_single_dir_creates_and_verifies() {
        let tmp = std::env::temp_dir().join("zquant_test_health");
        let _ = std::fs::remove_dir_all(&tmp);

        let result = ensure_single_dir("test", &tmp);
        assert_eq!(result.severity, CheckSeverity::Ok);
        assert!(tmp.exists());

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn path_init_error_display() {
        let e = PathInitError::EnvVarMissing("APPDATA".into());
        assert!(e.to_string().contains("APPDATA"));

        let e = PathInitError::CreateFailed("permission denied".into());
        assert!(e.to_string().contains("permission denied"));

        let e = PathInitError::NotWritable("C:\\readonly".into());
        assert!(e.to_string().contains("C:\\readonly"));
    }

    #[test]
    fn check_disk_space_does_not_panic() {
        let path = PathBuf::from("C:\\");
        let result = check_disk_space(&path);
        // Should not panic; severity is Ok or Warn
        assert!(result.severity == CheckSeverity::Ok || result.severity == CheckSeverity::Warn);
    }
}
