//! TOC building library for agent-memory.
//!
//! Provides:
//! - Event segmentation (TOC-03, TOC-04)
//! - Summarization trait (SUMM-01, SUMM-02, SUMM-04)
//! - TOC hierarchy building (TOC-01, TOC-02, TOC-05)

pub mod config;
pub mod segmenter;
pub mod summarizer;

pub use config::{SegmentationConfig, TocConfig};
pub use segmenter::{segment_events, SegmentBuilder, TokenCounter};
pub use summarizer::{ApiSummarizer, ApiSummarizerConfig, MockSummarizer, Summary, Summarizer, SummarizerError};
