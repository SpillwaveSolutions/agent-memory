//! Grip ID generation utilities.
//!
//! Grip IDs encode timestamp and ULID for ordering and uniqueness.

use chrono::{DateTime, Utc};

/// Generate a grip ID with timestamp and ULID.
///
/// Format: "grip:{timestamp_ms}:{ulid}"
///
/// The timestamp prefix enables time-ordered iteration.
pub fn generate_grip_id(timestamp: DateTime<Utc>) -> String {
    let timestamp_ms = timestamp.timestamp_millis();
    let ulid = ulid::Ulid::new();
    format!("grip:{}:{}", timestamp_ms, ulid)
}

/// Parse timestamp from grip ID.
pub fn parse_grip_timestamp(grip_id: &str) -> Option<DateTime<Utc>> {
    let parts: Vec<&str> = grip_id.split(':').collect();
    if parts.len() < 2 || parts[0] != "grip" {
        return None;
    }

    parts[1].parse::<i64>().ok()
        .and_then(|ms| chrono::DateTime::from_timestamp_millis(ms))
}

/// Check if a string is a valid grip ID format.
pub fn is_valid_grip_id(grip_id: &str) -> bool {
    let parts: Vec<&str> = grip_id.split(':').collect();
    if parts.len() != 3 || parts[0] != "grip" {
        return false;
    }

    // Check timestamp is numeric
    if parts[1].parse::<i64>().is_err() {
        return false;
    }

    // Check ULID format (26 characters, alphanumeric)
    parts[2].len() == 26 && parts[2].chars().all(|c| c.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_generate_grip_id() {
        let timestamp = Utc.with_ymd_and_hms(2024, 1, 29, 12, 0, 0).unwrap();
        let grip_id = generate_grip_id(timestamp);

        assert!(grip_id.starts_with("grip:1706529600000:"));
        assert!(is_valid_grip_id(&grip_id));
    }

    #[test]
    fn test_parse_grip_timestamp() {
        let timestamp = Utc.with_ymd_and_hms(2024, 1, 29, 12, 0, 0).unwrap();
        let grip_id = generate_grip_id(timestamp);

        let parsed = parse_grip_timestamp(&grip_id).unwrap();
        assert_eq!(parsed.timestamp_millis(), timestamp.timestamp_millis());
    }

    #[test]
    fn test_is_valid_grip_id() {
        // Generate a valid grip ID and test it
        let timestamp = Utc.with_ymd_and_hms(2024, 1, 29, 12, 0, 0).unwrap();
        let valid_id = generate_grip_id(timestamp);
        assert!(is_valid_grip_id(&valid_id));

        // Test invalid formats
        assert!(!is_valid_grip_id("invalid"));
        assert!(!is_valid_grip_id("grip:abc:123"));
        assert!(!is_valid_grip_id("toc:day:2024-01-29"));
        assert!(!is_valid_grip_id("grip:1706529600000:short"));
    }

    #[test]
    fn test_parse_invalid_grip_id() {
        assert!(parse_grip_timestamp("invalid").is_none());
        assert!(parse_grip_timestamp("grip:abc:123").is_none());
    }
}
