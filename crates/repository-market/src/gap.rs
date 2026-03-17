//! Gap calculation for time range queries.
//!
//! Identifies missing time ranges between requested range and available data.

use crate::Bar;
use chrono::{DateTime, Utc};

/// Time range gap
#[derive(Debug, Clone, PartialEq)]
pub struct Gap {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// Gap calculator
pub struct GapCalculator;

impl GapCalculator {
    /// Calculate gaps between requested range and available bars
    /// 
    /// Returns list of gaps that need to be filled from archive or remote.
    pub fn calculate_gaps(
        requested_start: DateTime<Utc>,
        requested_end: DateTime<Utc>,
        bars: &[Bar],
    ) -> Vec<Gap> {
        if bars.is_empty() {
            // Entire range is a gap
            return vec![Gap {
                start: requested_start,
                end: requested_end,
            }];
        }

        let mut gaps = Vec::new();
        let mut sorted_bars = bars.to_vec();
        sorted_bars.sort_by_key(|b| b.timestamp);

        // Check for gap before first bar
        let first_ts = sorted_bars[0].timestamp;
        if requested_start < first_ts {
            gaps.push(Gap {
                start: requested_start,
                end: first_ts,
            });
        }

        // Check for gaps between bars (not implemented for now - assumes continuous data)
        // This would require timeframe-aware gap detection

        // Check for gap after last bar
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

    fn create_test_bar(timestamp: DateTime<Utc>) -> Bar {
        Bar {
            timestamp,
            open: 100.0,
            high: 105.0,
            low: 99.0,
            close: 103.0,
            volume: 1000.0,
        }
    }

    #[test]
    fn calculate_gaps_empty_bars_returns_full_range() {
        let start = Utc.with_ymd_and_hms(2024, 3, 17, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 3, 18, 0, 0, 0).unwrap();

        let gaps = GapCalculator::calculate_gaps(start, end, &[]);

        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].start, start);
        assert_eq!(gaps[0].end, end);
    }

    #[test]
    fn calculate_gaps_finds_prefix_gap() {
        let start = Utc.with_ymd_and_hms(2024, 3, 17, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 3, 18, 0, 0, 0).unwrap();
        let bar_ts = Utc.with_ymd_and_hms(2024, 3, 17, 12, 0, 0).unwrap();

        let bars = vec![create_test_bar(bar_ts)];
        let gaps = GapCalculator::calculate_gaps(start, end, &bars);

        assert_eq!(gaps.len(), 2);
        assert_eq!(gaps[0].start, start);
        assert_eq!(gaps[0].end, bar_ts);
    }

    #[test]
    fn calculate_gaps_finds_suffix_gap() {
        let start = Utc.with_ymd_and_hms(2024, 3, 17, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 3, 18, 0, 0, 0).unwrap();
        let bar_ts = Utc.with_ymd_and_hms(2024, 3, 17, 12, 0, 0).unwrap();

        let bars = vec![create_test_bar(bar_ts)];
        let gaps = GapCalculator::calculate_gaps(start, end, &bars);

        assert_eq!(gaps.len(), 2);
        assert_eq!(gaps[1].start, bar_ts);
        assert_eq!(gaps[1].end, end);
    }

    #[test]
    fn calculate_gaps_no_gaps_when_covered() {
        let start = Utc.with_ymd_and_hms(2024, 3, 17, 12, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 3, 17, 13, 0, 0).unwrap();
        
        let bars = vec![
            create_test_bar(start),
            create_test_bar(end),
        ];
        
        let gaps = GapCalculator::calculate_gaps(start, end, &bars);

        assert_eq!(gaps.len(), 0);
    }
}
