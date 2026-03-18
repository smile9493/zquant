//! Gap calculation for time range queries.
//!
//! Identifies missing time ranges between requested range and available data,
//! including prefix, middle (inter-bar), and suffix gaps.

use crate::Bar;
use chrono::{DateTime, Duration, Utc};

/// Time range gap
#[derive(Debug, Clone, PartialEq)]
pub struct Gap {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// Parse a timeframe string into a `chrono::Duration`.
///
/// Returns `None` for unrecognized formats, allowing callers to
/// fall back to prefix/suffix-only detection.
fn parse_timeframe(timeframe: &str) -> Option<Duration> {
    match timeframe {
        "1d" | "daily" => Some(Duration::hours(24)),
        "4h" => Some(Duration::hours(4)),
        "1h" => Some(Duration::hours(1)),
        _ => None,
    }
}

/// Gap calculator
pub struct GapCalculator;

impl GapCalculator {
    /// Calculate gaps between requested range and available bars.
    ///
    /// When `timeframe` resolves to a known duration, middle gaps between
    /// consecutive bars are detected (threshold = 1.5× expected interval).
    /// Unknown timeframes fall back to prefix + suffix detection only.
    pub fn calculate_gaps(
        requested_start: DateTime<Utc>,
        requested_end: DateTime<Utc>,
        bars: &[Bar],
        timeframe: &str,
    ) -> Vec<Gap> {
        if bars.is_empty() {
            return vec![Gap {
                start: requested_start,
                end: requested_end,
            }];
        }

        let mut sorted_bars = bars.to_vec();
        sorted_bars.sort_by_key(|b| b.timestamp);

        let mut gaps = Vec::new();

        // Prefix gap
        let first_ts = sorted_bars[0].timestamp;
        if requested_start < first_ts {
            gaps.push(Gap {
                start: requested_start,
                end: first_ts,
            });
        }

        // Middle gaps (only when timeframe is known)
        if let Some(expected) = parse_timeframe(timeframe) {
            // Use 1.5x threshold to tolerate minor timestamp jitter
            let threshold = expected + expected / 2;
            for window in sorted_bars.windows(2) {
                let delta = window[1].timestamp - window[0].timestamp;
                if delta > threshold {
                    gaps.push(Gap {
                        start: window[0].timestamp,
                        end: window[1].timestamp,
                    });
                }
            }
        }

        // Suffix gap
        let last_ts = sorted_bars[sorted_bars.len() - 1].timestamp;
        if last_ts < requested_end {
            gaps.push(Gap {
                start: last_ts,
                end: requested_end,
            });
        }

        gaps
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn bar_at(ts: DateTime<Utc>) -> Bar {
        Bar {
            timestamp: ts,
            open: 100.0,
            high: 105.0,
            low: 99.0,
            close: 103.0,
            volume: 1000.0,
        }
    }

    // --- Existing behavior (prefix / suffix / empty / covered) ---

    #[test]
    fn calculate_gaps_empty_bars_returns_full_range() {
        let start = Utc.with_ymd_and_hms(2024, 3, 17, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 3, 18, 0, 0, 0).unwrap();

        let gaps = GapCalculator::calculate_gaps(start, end, &[], "1d");

        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].start, start);
        assert_eq!(gaps[0].end, end);
    }

    #[test]
    fn calculate_gaps_finds_prefix_gap() {
        let start = Utc.with_ymd_and_hms(2024, 3, 17, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 3, 18, 0, 0, 0).unwrap();
        let bar_ts = Utc.with_ymd_and_hms(2024, 3, 17, 12, 0, 0).unwrap();

        let bars = vec![bar_at(bar_ts)];
        let gaps = GapCalculator::calculate_gaps(start, end, &bars, "1d");

        // prefix + suffix
        assert_eq!(gaps.len(), 2);
        assert_eq!(gaps[0].start, start);
        assert_eq!(gaps[0].end, bar_ts);
    }

    #[test]
    fn calculate_gaps_finds_suffix_gap() {
        let start = Utc.with_ymd_and_hms(2024, 3, 17, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 3, 18, 0, 0, 0).unwrap();
        let bar_ts = Utc.with_ymd_and_hms(2024, 3, 17, 12, 0, 0).unwrap();

        let bars = vec![bar_at(bar_ts)];
        let gaps = GapCalculator::calculate_gaps(start, end, &bars, "1d");

        assert_eq!(gaps.len(), 2);
        assert_eq!(gaps[1].start, bar_ts);
        assert_eq!(gaps[1].end, end);
    }

    #[test]
    fn calculate_gaps_no_gaps_when_covered() {
        let start = Utc.with_ymd_and_hms(2024, 3, 17, 12, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 3, 17, 13, 0, 0).unwrap();

        let bars = vec![bar_at(start), bar_at(end)];
        let gaps = GapCalculator::calculate_gaps(start, end, &bars, "1h");

        assert_eq!(gaps.len(), 0);
    }

    // --- Middle gap detection ---

    #[test]
    fn calculate_gaps_detects_single_middle_gap_daily() {
        // 3 daily bars with a 3-day hole in the middle
        let d1 = Utc.with_ymd_and_hms(2024, 3, 1, 0, 0, 0).unwrap();
        let d2 = Utc.with_ymd_and_hms(2024, 3, 2, 0, 0, 0).unwrap();
        // gap: 3/2 -> 3/5
        let d5 = Utc.with_ymd_and_hms(2024, 3, 5, 0, 0, 0).unwrap();
        let d6 = Utc.with_ymd_and_hms(2024, 3, 6, 0, 0, 0).unwrap();

        let bars = vec![bar_at(d1), bar_at(d2), bar_at(d5), bar_at(d6)];
        let gaps = GapCalculator::calculate_gaps(d1, d6, &bars, "1d");

        // Only middle gap (d2 -> d5); no prefix (first bar == start), no suffix (last bar == end)
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].start, d2);
        assert_eq!(gaps[0].end, d5);
    }

    #[test]
    fn calculate_gaps_detects_multiple_middle_gaps() {
        let d1 = Utc.with_ymd_and_hms(2024, 3, 1, 0, 0, 0).unwrap();
        let d4 = Utc.with_ymd_and_hms(2024, 3, 4, 0, 0, 0).unwrap();
        let d8 = Utc.with_ymd_and_hms(2024, 3, 8, 0, 0, 0).unwrap();

        let bars = vec![bar_at(d1), bar_at(d4), bar_at(d8)];
        let gaps = GapCalculator::calculate_gaps(d1, d8, &bars, "1d");

        // Two middle gaps: d1->d4, d4->d8
        assert_eq!(gaps.len(), 2);
        assert_eq!(gaps[0].start, d1);
        assert_eq!(gaps[0].end, d4);
        assert_eq!(gaps[1].start, d4);
        assert_eq!(gaps[1].end, d8);
    }

    #[test]
    fn calculate_gaps_no_middle_gap_when_continuous_daily() {
        let d1 = Utc.with_ymd_and_hms(2024, 3, 1, 0, 0, 0).unwrap();
        let d2 = Utc.with_ymd_and_hms(2024, 3, 2, 0, 0, 0).unwrap();
        let d3 = Utc.with_ymd_and_hms(2024, 3, 3, 0, 0, 0).unwrap();

        let bars = vec![bar_at(d1), bar_at(d2), bar_at(d3)];
        let gaps = GapCalculator::calculate_gaps(d1, d3, &bars, "1d");

        assert_eq!(gaps.len(), 0);
    }

    #[test]
    fn calculate_gaps_hourly_middle_gap() {
        let h0 = Utc.with_ymd_and_hms(2024, 3, 1, 0, 0, 0).unwrap();
        let h1 = Utc.with_ymd_and_hms(2024, 3, 1, 1, 0, 0).unwrap();
        // gap: 3h missing
        let h5 = Utc.with_ymd_and_hms(2024, 3, 1, 5, 0, 0).unwrap();

        let bars = vec![bar_at(h0), bar_at(h1), bar_at(h5)];
        let gaps = GapCalculator::calculate_gaps(h0, h5, &bars, "1h");

        // middle gap h1->h5
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].start, h1);
        assert_eq!(gaps[0].end, h5);
    }

    #[test]
    fn calculate_gaps_unknown_timeframe_skips_middle_detection() {
        let d1 = Utc.with_ymd_and_hms(2024, 3, 1, 0, 0, 0).unwrap();
        let d5 = Utc.with_ymd_and_hms(2024, 3, 5, 0, 0, 0).unwrap();
        let d10 = Utc.with_ymd_and_hms(2024, 3, 10, 0, 0, 0).unwrap();

        let bars = vec![bar_at(d1), bar_at(d5), bar_at(d10)];
        // Unknown timeframe -> no middle gap detection
        let gaps = GapCalculator::calculate_gaps(d1, d10, &bars, "tick");

        // Only no prefix (d1==start), no suffix (d10==end), no middle (unknown tf)
        assert_eq!(gaps.len(), 0);
    }

    #[test]
    fn calculate_gaps_prefix_middle_suffix_combined() {
        let req_start = Utc.with_ymd_and_hms(2024, 3, 1, 0, 0, 0).unwrap();
        let d3 = Utc.with_ymd_and_hms(2024, 3, 3, 0, 0, 0).unwrap();
        let d7 = Utc.with_ymd_and_hms(2024, 3, 7, 0, 0, 0).unwrap();
        let req_end = Utc.with_ymd_and_hms(2024, 3, 10, 0, 0, 0).unwrap();

        let bars = vec![bar_at(d3), bar_at(d7)];
        let gaps = GapCalculator::calculate_gaps(req_start, req_end, &bars, "1d");

        // prefix: req_start->d3, middle: d3->d7, suffix: d7->req_end
        assert_eq!(gaps.len(), 3);
        assert_eq!(gaps[0].start, req_start);
        assert_eq!(gaps[0].end, d3);
        assert_eq!(gaps[1].start, d3);
        assert_eq!(gaps[1].end, d7);
        assert_eq!(gaps[2].start, d7);
        assert_eq!(gaps[2].end, req_end);
    }

    // --- parse_timeframe ---

    #[test]
    fn parse_timeframe_known_values() {
        assert_eq!(parse_timeframe("1d"), Some(Duration::hours(24)));
        assert_eq!(parse_timeframe("daily"), Some(Duration::hours(24)));
        assert_eq!(parse_timeframe("4h"), Some(Duration::hours(4)));
        assert_eq!(parse_timeframe("1h"), Some(Duration::hours(1)));
    }

    #[test]
    fn parse_timeframe_unknown_returns_none() {
        assert_eq!(parse_timeframe("tick"), None);
        assert_eq!(parse_timeframe("5m"), None);
        assert_eq!(parse_timeframe(""), None);
    }
}
