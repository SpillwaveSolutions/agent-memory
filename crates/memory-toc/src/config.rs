//! Configuration for TOC building.

use serde::{Deserialize, Serialize};

/// Configuration for event segmentation.
///
/// Per TOC-03: Segment on time threshold (30 min) or token threshold (4K).
/// Per TOC-04: Overlap for context continuity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentationConfig {
    /// Maximum time gap before starting new segment (milliseconds)
    /// Per TOC-03: Default 30 minutes
    pub time_threshold_ms: i64,

    /// Maximum tokens before starting new segment
    /// Per TOC-03: Default 4000 tokens
    pub token_threshold: usize,

    /// Overlap time to include from previous segment (milliseconds)
    /// Per TOC-04: Default 5 minutes
    pub overlap_time_ms: i64,

    /// Overlap tokens to include from previous segment
    /// Per TOC-04: Default 500 tokens
    pub overlap_tokens: usize,

    /// Maximum text length to count for tool results (to avoid explosion)
    pub max_tool_result_chars: usize,
}

impl Default for SegmentationConfig {
    fn default() -> Self {
        Self {
            time_threshold_ms: 30 * 60 * 1000, // 30 minutes
            token_threshold: 4000,
            overlap_time_ms: 5 * 60 * 1000, // 5 minutes
            overlap_tokens: 500,
            max_tool_result_chars: 1000,
        }
    }
}

/// Overall TOC configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocConfig {
    /// Segmentation settings
    pub segmentation: SegmentationConfig,

    /// Minimum events to create a segment
    pub min_events_per_segment: usize,
}

impl Default for TocConfig {
    fn default() -> Self {
        Self {
            segmentation: SegmentationConfig::default(),
            min_events_per_segment: 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SegmentationConfig::default();
        assert_eq!(config.time_threshold_ms, 30 * 60 * 1000);
        assert_eq!(config.token_threshold, 4000);
        assert_eq!(config.overlap_time_ms, 5 * 60 * 1000);
        assert_eq!(config.overlap_tokens, 500);
    }

    #[test]
    fn test_config_serialization() {
        let config = TocConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let decoded: TocConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(
            config.segmentation.token_threshold,
            decoded.segmentation.token_threshold
        );
    }
}
