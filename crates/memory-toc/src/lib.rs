//! TOC building library for agent-memory.
//!
//! Provides:
//! - Event segmentation (TOC-03, TOC-04)
//! - Summarization trait (SUMM-01, SUMM-02, SUMM-04)
//! - TOC hierarchy building (TOC-01, TOC-02, TOC-05)
//! - Node ID generation
//! - Grip ID generation and provenance
//! - Grip expansion for context retrieval (GRIP-04)

pub mod builder;
pub mod config;
pub mod expand;
pub mod grip_id;
pub mod node_id;
pub mod rollup;
pub mod segmenter;
pub mod summarizer;

pub use builder::{BuilderError, TocBuilder};
pub use config::{SegmentationConfig, TocConfig};
pub use expand::{expand_grip, ExpandConfig, ExpandedGrip, ExpandError, GripExpander};
pub use grip_id::{generate_grip_id, is_valid_grip_id, parse_grip_timestamp};
pub use node_id::{generate_node_id, generate_title, get_parent_node_id, parse_level};
pub use rollup::{RollupCheckpoint, RollupError, RollupJob, run_all_rollups};
pub use segmenter::{segment_events, SegmentBuilder, TokenCounter};
pub use summarizer::{ApiSummarizer, ApiSummarizerConfig, MockSummarizer, Summary, Summarizer, SummarizerError};
