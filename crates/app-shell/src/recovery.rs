//! Startup strategy classification based on health check results.
//!
//! Determines whether the application should continue normally,
//! run in degraded mode, or block startup based on check severity.

use crate::health::{CheckSeverity, HealthReport};

/// Startup strategy derived from health check results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupStrategy {
    /// All checks passed — full functionality available.
    Continue,
    /// Some checks have warnings — app runs with reduced features.
    Degrade,
    /// Critical checks failed — app should show error UI only.
    Block,
}

/// Determine the startup strategy from a health report.
///
/// Rules:
/// - Any `Error` on a directory check → `Block` (cannot persist state)
/// - Any other `Error` → `Degrade` (e.g. DB down, still usable in UI-only)
/// - Any `Warn` without errors → `Degrade`
/// - All `Ok` → `Continue`
pub fn determine_startup_strategy(report: &HealthReport) -> StartupStrategy {
    let has_dir_error = report
        .results
        .iter()
        .any(|r| r.severity == CheckSeverity::Error && r.name.starts_with("dir:"));

    if has_dir_error {
        return StartupStrategy::Block;
    }

    if report.has_errors() || report.has_warnings() {
        return StartupStrategy::Degrade;
    }

    StartupStrategy::Continue
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::{CheckResult, HealthReport};
    use chrono::Utc;

    fn make_result(name: &str, severity: CheckSeverity) -> CheckResult {
        CheckResult {
            name: name.into(),
            severity,
            message: "test".into(),
            checked_at: Utc::now(),
        }
    }

    fn make_report(results: Vec<CheckResult>) -> HealthReport {
        HealthReport {
            results,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn all_ok_returns_continue() {
        let report = make_report(vec![
            make_result("dir:config_dir", CheckSeverity::Ok),
            make_result("db:connectivity", CheckSeverity::Ok),
        ]);
        assert_eq!(determine_startup_strategy(&report), StartupStrategy::Continue);
    }

    #[test]
    fn warn_only_returns_degrade() {
        let report = make_report(vec![
            make_result("dir:config_dir", CheckSeverity::Ok),
            make_result("db:connectivity", CheckSeverity::Warn),
        ]);
        assert_eq!(determine_startup_strategy(&report), StartupStrategy::Degrade);
    }

    #[test]
    fn dir_error_returns_block() {
        let report = make_report(vec![
            make_result("dir:config_dir", CheckSeverity::Error),
            make_result("db:connectivity", CheckSeverity::Ok),
        ]);
        assert_eq!(determine_startup_strategy(&report), StartupStrategy::Block);
    }

    #[test]
    fn non_dir_error_returns_degrade() {
        let report = make_report(vec![
            make_result("dir:config_dir", CheckSeverity::Ok),
            make_result("disk:space", CheckSeverity::Error),
        ]);
        assert_eq!(determine_startup_strategy(&report), StartupStrategy::Degrade);
    }

    #[test]
    fn empty_report_returns_continue() {
        let report = make_report(vec![]);
        assert_eq!(determine_startup_strategy(&report), StartupStrategy::Continue);
    }
}
