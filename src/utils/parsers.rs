use chrono::{NaiveDate, NaiveDateTime};

pub fn parse_cutoff_date(input: &str) -> Result<NaiveDateTime, String> {
    if let Ok(dt) = NaiveDateTime::parse_from_str(input, "%Y.%m.%d:%H.%M.%S") {
        return Ok(dt);
    }

    // Then try just date (default to midnight)
    if let Ok(date) = NaiveDate::parse_from_str(input, "%Y.%m.%d") {
        return Ok(date.and_hms_opt(0, 0, 0).unwrap());
    }

    Err(format!(
        "Invalid date format: '{}'. Expected 'YYYY.MM.DD[:HH24.MI.SS]'",
        input
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_parse_cutoff_date_with_full_datetime() {
        let input = "2025.11.07:12.34.56";
        let result = parse_cutoff_date(input).unwrap();
        let expected = NaiveDate::from_ymd_opt(2025, 11, 7)
            .unwrap()
            .and_hms_opt(12, 34, 56)
            .unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parse_cutoff_date_with_only_date() {
        let input = "2025.11.07";
        let result = parse_cutoff_date(input).unwrap();
        let expected = NaiveDate::from_ymd_opt(2025, 11, 7)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parse_cutoff_date_invalid_format() {
        let input = "2025-11-07"; // wrong format
        let result = parse_cutoff_date(input);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Invalid date format: '2025-11-07'")
        );
    }
}
