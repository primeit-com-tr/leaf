use chrono::{DateTime, Utc};

/// Formats the duration between two optional timestamps as a human-readable string.
///
/// Example output: `"0.148s"`
pub fn format_duration(
    started_at: Option<DateTime<Utc>>,
    ended_at: Option<DateTime<Utc>>,
) -> String {
    started_at
        .zip(ended_at)
        .map(|(s, e)| {
            let duration = e - s;
            format!("{:.3}s", duration.num_milliseconds() as f64 / 1000.0)
        })
        .unwrap_or_else(|| "0.000s".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn test_duration_normal_case() {
        let started = Utc.with_ymd_and_hms(2025, 11, 7, 10, 0, 0).unwrap();
        let ended = Utc.with_ymd_and_hms(2025, 11, 7, 10, 0, 1).unwrap();
        let result = format_duration(Some(started), Some(ended));
        assert_eq!(result, "1.000s");
    }

    #[test]
    fn test_duration_with_milliseconds() {
        let started = Utc.timestamp_millis_opt(1_000).unwrap();
        let ended = Utc.timestamp_millis_opt(1_148).unwrap();
        let result = format_duration(Some(started), Some(ended));
        assert_eq!(result, "0.148s");
    }

    #[test]
    fn test_duration_with_none_start() {
        let ended = Utc.with_ymd_and_hms(2025, 11, 7, 10, 0, 1).unwrap();
        let result = format_duration(None, Some(ended));
        assert_eq!(result, "0.000s");
    }

    #[test]
    fn test_duration_with_none_end() {
        let started = Utc.with_ymd_and_hms(2025, 11, 7, 10, 0, 0).unwrap();
        let result = format_duration(Some(started), None);
        assert_eq!(result, "0.000s");
    }

    #[test]
    fn test_duration_with_both_none() {
        let result = format_duration(None, None);
        assert_eq!(result, "0.000s");
    }

    #[test]
    fn test_duration_reversed_timestamps() {
        // End before start — should show a negative duration
        let started = Utc.with_ymd_and_hms(2025, 11, 7, 10, 0, 1).unwrap();
        let ended = Utc.with_ymd_and_hms(2025, 11, 7, 10, 0, 0).unwrap();
        let result = format_duration(Some(started), Some(ended));
        assert_eq!(result, "-1.000s");
    }
}
