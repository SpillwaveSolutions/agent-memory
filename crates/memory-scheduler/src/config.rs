//! Scheduler configuration.
//!
//! Provides configuration for the scheduler service including
//! default timezone and shutdown timeout settings.

use serde::{Deserialize, Serialize};

use crate::SchedulerError;

/// Configuration for the scheduler service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    /// Default timezone for jobs (IANA timezone string, e.g., "America/New_York").
    /// Defaults to "UTC".
    #[serde(default = "default_timezone")]
    pub default_timezone: String,

    /// Timeout in seconds for graceful shutdown.
    /// Jobs will be given this much time to complete before forced termination.
    /// Defaults to 30 seconds.
    #[serde(default = "default_shutdown_timeout")]
    pub shutdown_timeout_secs: u64,
}

fn default_timezone() -> String {
    "UTC".to_string()
}

fn default_shutdown_timeout() -> u64 {
    30
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            default_timezone: default_timezone(),
            shutdown_timeout_secs: default_shutdown_timeout(),
        }
    }
}

impl SchedulerConfig {
    /// Parse the configured timezone string into a chrono_tz::Tz.
    ///
    /// # Errors
    ///
    /// Returns `SchedulerError::InvalidTimezone` if the timezone string
    /// is not a valid IANA timezone identifier.
    pub fn parse_timezone(&self) -> Result<chrono_tz::Tz, SchedulerError> {
        self.default_timezone
            .parse::<chrono_tz::Tz>()
            .map_err(|_| SchedulerError::InvalidTimezone(self.default_timezone.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SchedulerConfig::default();
        assert_eq!(config.default_timezone, "UTC");
        assert_eq!(config.shutdown_timeout_secs, 30);
    }

    #[test]
    fn test_parse_timezone_utc() {
        let config = SchedulerConfig::default();
        let tz = config.parse_timezone().unwrap();
        assert_eq!(tz.name(), "UTC");
    }

    #[test]
    fn test_parse_timezone_america_new_york() {
        let config = SchedulerConfig {
            default_timezone: "America/New_York".to_string(),
            ..Default::default()
        };
        let tz = config.parse_timezone().unwrap();
        assert_eq!(tz.name(), "America/New_York");
    }

    #[test]
    fn test_parse_invalid_timezone() {
        let config = SchedulerConfig {
            default_timezone: "Invalid/Zone".to_string(),
            ..Default::default()
        };
        let result = config.parse_timezone();
        assert!(result.is_err());
        match result {
            Err(SchedulerError::InvalidTimezone(tz)) => assert_eq!(tz, "Invalid/Zone"),
            _ => panic!("Expected InvalidTimezone error"),
        }
    }

    #[test]
    fn test_serde_roundtrip() {
        let config = SchedulerConfig {
            default_timezone: "Europe/London".to_string(),
            shutdown_timeout_secs: 60,
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: SchedulerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.default_timezone, "Europe/London");
        assert_eq!(parsed.shutdown_timeout_secs, 60);
    }
}
