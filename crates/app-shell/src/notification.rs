//! User-facing actionable fix suggestions derived from health check results.
//!
//! Translates technical `CheckResult` items into human-readable messages
//! that tell the user what went wrong and how to fix it.

use crate::health::{CheckResult, CheckSeverity, HealthReport};

/// A user-facing notification with an actionable suggestion.
#[derive(Debug, Clone)]
pub struct StartupNotification {
    /// Short title for the issue.
    pub title: String,
    /// Actionable suggestion the user can follow.
    pub suggestion: String,
    /// Severity carried from the original check.
    pub severity: CheckSeverity,
}

/// Generate user-facing notifications from a health report.
/// Only produces notifications for non-Ok results.
pub fn generate_notifications(report: &HealthReport) -> Vec<StartupNotification> {
    report
        .results
        .iter()
        .filter(|r| r.severity != CheckSeverity::Ok)
        .map(|r| to_notification(r))
        .collect()
}

fn to_notification(result: &CheckResult) -> StartupNotification {
    let (title, suggestion) = match result.name.as_str() {
        "db:connectivity" => (
            "数据库连接异常".into(),
            format!(
                "请检查 PostgreSQL 是否正在运行，并确认 DATABASE_URL 环境变量配置正确。详情：{}",
                result.message
            ),
        ),
        name if name.starts_with("dir:") => {
            let dir_label = name.strip_prefix("dir:").unwrap_or(name);
            (
                format!("目录 {dir_label} 异常"),
                format!(
                    "请检查目录权限或手动创建该目录。详情：{}",
                    result.message
                ),
            )
        }
        "disk:space" => (
            "磁盘空间不足".into(),
            format!(
                "请清理磁盘空间或将数据目录迁移到更大的分区。详情：{}",
                result.message
            ),
        ),
        _ => (
            format!("检查项 {} 异常", result.name),
            format!("详情：{}", result.message),
        ),
    };

    StartupNotification {
        title,
        suggestion,
        severity: result.severity,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::{CheckResult, HealthReport};
    use chrono::Utc;

    fn make_result(name: &str, severity: CheckSeverity, msg: &str) -> CheckResult {
        CheckResult {
            name: name.into(),
            severity,
            message: msg.into(),
            checked_at: Utc::now(),
        }
    }

    #[test]
    fn ok_results_produce_no_notifications() {
        let report = HealthReport {
            results: vec![make_result("db:connectivity", CheckSeverity::Ok, "connected")],
            created_at: Utc::now(),
        };
        assert!(generate_notifications(&report).is_empty());
    }

    #[test]
    fn db_warn_produces_db_notification() {
        let report = HealthReport {
            results: vec![make_result(
                "db:connectivity",
                CheckSeverity::Warn,
                "connection refused",
            )],
            created_at: Utc::now(),
        };
        let notes = generate_notifications(&report);
        assert_eq!(notes.len(), 1);
        assert!(notes[0].title.contains("数据库"));
        assert!(notes[0].suggestion.contains("PostgreSQL"));
    }

    #[test]
    fn dir_error_produces_dir_notification() {
        let report = HealthReport {
            results: vec![make_result(
                "dir:config_dir",
                CheckSeverity::Error,
                "permission denied",
            )],
            created_at: Utc::now(),
        };
        let notes = generate_notifications(&report);
        assert_eq!(notes.len(), 1);
        assert!(notes[0].title.contains("config_dir"));
        assert!(notes[0].suggestion.contains("权限"));
    }

    #[test]
    fn disk_warn_produces_disk_notification() {
        let report = HealthReport {
            results: vec![make_result(
                "disk:space",
                CheckSeverity::Warn,
                "200 MB free",
            )],
            created_at: Utc::now(),
        };
        let notes = generate_notifications(&report);
        assert_eq!(notes.len(), 1);
        assert!(notes[0].title.contains("磁盘"));
    }

    #[test]
    fn mixed_results_filter_ok() {
        let report = HealthReport {
            results: vec![
                make_result("dir:config_dir", CheckSeverity::Ok, "ok"),
                make_result("db:connectivity", CheckSeverity::Warn, "timeout"),
                make_result("disk:space", CheckSeverity::Ok, "plenty"),
            ],
            created_at: Utc::now(),
        };
        let notes = generate_notifications(&report);
        assert_eq!(notes.len(), 1);
        assert!(notes[0].title.contains("数据库"));
    }
}
