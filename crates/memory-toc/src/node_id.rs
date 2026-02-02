//! TOC node ID generation and parsing.
//!
//! Node IDs encode the level and time period for hierarchical organization.
//! Format: "toc:{level}:{time_identifier}"

use chrono::{DateTime, Datelike, Utc, Weekday};
use memory_types::TocLevel;

/// Generate a node ID for the given level and time.
///
/// Examples:
/// - Year: "toc:year:2024"
/// - Month: "toc:month:2024:01"
/// - Week: "toc:week:2024:W03"
/// - Day: "toc:day:2024-01-15"
/// - Segment: "toc:segment:2024-01-15:01HN4QXKN6..."
pub fn generate_node_id(level: TocLevel, time: DateTime<Utc>) -> String {
    match level {
        TocLevel::Year => format!("toc:year:{}", time.year()),
        TocLevel::Month => format!("toc:month:{}:{:02}", time.year(), time.month()),
        TocLevel::Week => {
            let iso_week = time.iso_week();
            format!("toc:week:{}:W{:02}", iso_week.year(), iso_week.week())
        }
        TocLevel::Day => format!("toc:day:{}", time.format("%Y-%m-%d")),
        TocLevel::Segment => format!(
            "toc:segment:{}:{}",
            time.format("%Y-%m-%d"),
            ulid::Ulid::new()
        ),
    }
}

/// Generate a node ID for a segment with a specific ULID.
pub fn generate_segment_node_id(time: DateTime<Utc>, segment_ulid: &str) -> String {
    format!("toc:segment:{}:{}", time.format("%Y-%m-%d"), segment_ulid)
}

/// Get the parent node ID for a given node ID.
///
/// Returns None for year-level nodes (no parent).
pub fn get_parent_node_id(node_id: &str) -> Option<String> {
    let parts: Vec<&str> = node_id.split(':').collect();
    if parts.len() < 3 || parts[0] != "toc" {
        return None;
    }

    match parts[1] {
        "segment" => {
            // toc:segment:2024-01-15:ulid -> toc:day:2024-01-15
            if parts.len() >= 3 {
                Some(format!("toc:day:{}", parts[2]))
            } else {
                None
            }
        }
        "day" => {
            // toc:day:2024-01-15 -> toc:week:2024:W03
            if parts.len() >= 3 {
                if let Ok(date) = chrono::NaiveDate::parse_from_str(parts[2], "%Y-%m-%d") {
                    let iso_week = date.iso_week();
                    return Some(format!(
                        "toc:week:{}:W{:02}",
                        iso_week.year(),
                        iso_week.week()
                    ));
                }
            }
            None
        }
        "week" => {
            // toc:week:2024:W03 -> toc:month:2024:01
            // Need to find which month the week belongs to (use middle of week)
            if parts.len() >= 4 {
                if let (Ok(year), Ok(week)) = (
                    parts[2].parse::<i32>(),
                    parts[3].trim_start_matches('W').parse::<u32>(),
                ) {
                    // Get the Thursday of the week to determine the month
                    if let Some(date) = chrono::NaiveDate::from_isoywd_opt(year, week, Weekday::Thu)
                    {
                        return Some(format!("toc:month:{}:{:02}", date.year(), date.month()));
                    }
                }
            }
            None
        }
        "month" => {
            // toc:month:2024:01 -> toc:year:2024
            if parts.len() >= 3 {
                Some(format!("toc:year:{}", parts[2]))
            } else {
                None
            }
        }
        "year" => None, // Year has no parent
        _ => None,
    }
}

/// Parse level from node ID.
pub fn parse_level(node_id: &str) -> Option<TocLevel> {
    let parts: Vec<&str> = node_id.split(':').collect();
    if parts.len() < 2 || parts[0] != "toc" {
        return None;
    }

    match parts[1] {
        "year" => Some(TocLevel::Year),
        "month" => Some(TocLevel::Month),
        "week" => Some(TocLevel::Week),
        "day" => Some(TocLevel::Day),
        "segment" => Some(TocLevel::Segment),
        _ => None,
    }
}

/// Generate human-readable title for a node.
pub fn generate_title(level: TocLevel, time: DateTime<Utc>) -> String {
    match level {
        TocLevel::Year => format!("{}", time.year()),
        TocLevel::Month => time.format("%B %Y").to_string(),
        TocLevel::Week => {
            let iso_week = time.iso_week();
            format!("Week {} of {}", iso_week.week(), iso_week.year())
        }
        TocLevel::Day => time.format("%A, %B %d, %Y").to_string(),
        TocLevel::Segment => time.format("%B %d, %Y at %H:%M").to_string(),
    }
}

/// Get the time boundaries for a level at a given time.
pub fn get_time_boundaries(level: TocLevel, time: DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>) {
    use chrono::{Duration, NaiveTime, TimeZone};

    match level {
        TocLevel::Year => {
            let start = Utc.with_ymd_and_hms(time.year(), 1, 1, 0, 0, 0).unwrap();
            let end = Utc
                .with_ymd_and_hms(time.year() + 1, 1, 1, 0, 0, 0)
                .unwrap()
                - Duration::milliseconds(1);
            (start, end)
        }
        TocLevel::Month => {
            let start = Utc
                .with_ymd_and_hms(time.year(), time.month(), 1, 0, 0, 0)
                .unwrap();
            let next_month = if time.month() == 12 {
                Utc.with_ymd_and_hms(time.year() + 1, 1, 1, 0, 0, 0)
                    .unwrap()
            } else {
                Utc.with_ymd_and_hms(time.year(), time.month() + 1, 1, 0, 0, 0)
                    .unwrap()
            };
            let end = next_month - Duration::milliseconds(1);
            (start, end)
        }
        TocLevel::Week => {
            let iso_week = time.iso_week();
            let monday =
                chrono::NaiveDate::from_isoywd_opt(iso_week.year(), iso_week.week(), Weekday::Mon)
                    .unwrap();
            let start = Utc.from_utc_datetime(&monday.and_time(NaiveTime::MIN));
            let end = start + Duration::days(7) - Duration::milliseconds(1);
            (start, end)
        }
        TocLevel::Day => {
            let date = time.date_naive();
            let start = Utc.from_utc_datetime(&date.and_time(NaiveTime::MIN));
            let end = start + Duration::days(1) - Duration::milliseconds(1);
            (start, end)
        }
        TocLevel::Segment => {
            // Segments have explicit boundaries, not calculated
            (time, time)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_generate_node_id_year() {
        let time = Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap();
        let id = generate_node_id(TocLevel::Year, time);
        assert_eq!(id, "toc:year:2024");
    }

    #[test]
    fn test_generate_node_id_month() {
        let time = Utc.with_ymd_and_hms(2024, 1, 15, 12, 0, 0).unwrap();
        let id = generate_node_id(TocLevel::Month, time);
        assert_eq!(id, "toc:month:2024:01");
    }

    #[test]
    fn test_generate_node_id_week() {
        let time = Utc.with_ymd_and_hms(2024, 1, 18, 12, 0, 0).unwrap();
        let id = generate_node_id(TocLevel::Week, time);
        assert!(id.starts_with("toc:week:2024:W"));
    }

    #[test]
    fn test_generate_node_id_day() {
        let time = Utc.with_ymd_and_hms(2024, 1, 15, 12, 0, 0).unwrap();
        let id = generate_node_id(TocLevel::Day, time);
        assert_eq!(id, "toc:day:2024-01-15");
    }

    #[test]
    fn test_generate_node_id_segment() {
        let time = Utc.with_ymd_and_hms(2024, 1, 15, 12, 0, 0).unwrap();
        let id = generate_node_id(TocLevel::Segment, time);
        assert!(id.starts_with("toc:segment:2024-01-15:"));
    }

    #[test]
    fn test_get_parent_node_id() {
        assert_eq!(
            get_parent_node_id("toc:day:2024-01-15"),
            Some("toc:week:2024:W03".to_string())
        );
        assert_eq!(
            get_parent_node_id("toc:month:2024:01"),
            Some("toc:year:2024".to_string())
        );
        assert_eq!(get_parent_node_id("toc:year:2024"), None);
    }

    #[test]
    fn test_parse_level() {
        assert_eq!(parse_level("toc:year:2024"), Some(TocLevel::Year));
        assert_eq!(parse_level("toc:month:2024:01"), Some(TocLevel::Month));
        assert_eq!(parse_level("toc:day:2024-01-15"), Some(TocLevel::Day));
        assert_eq!(parse_level("invalid"), None);
    }

    #[test]
    fn test_generate_title() {
        let time = Utc.with_ymd_and_hms(2024, 1, 15, 12, 0, 0).unwrap();
        assert_eq!(generate_title(TocLevel::Year, time), "2024");
        assert_eq!(generate_title(TocLevel::Month, time), "January 2024");
    }

    #[test]
    fn test_get_time_boundaries_day() {
        let time = Utc.with_ymd_and_hms(2024, 1, 15, 12, 30, 0).unwrap();
        let (start, end) = get_time_boundaries(TocLevel::Day, time);

        assert_eq!(start, Utc.with_ymd_and_hms(2024, 1, 15, 0, 0, 0).unwrap());
        assert!(end > start);
        assert!(end < Utc.with_ymd_and_hms(2024, 1, 16, 0, 0, 0).unwrap());
    }
}
