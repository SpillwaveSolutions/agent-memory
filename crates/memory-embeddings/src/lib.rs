//! # memory-embeddings
//!
//! Local embedding generation for Agent Memory using Candle.
//!
//! This crate provides semantic vector embeddings for TOC summaries and grip
//! excerpts, enabling similarity search without external API calls.
//!
//! ## Features
//! - Local inference via Candle (no Python, no API)
//! - all-MiniLM-L6-v2 model (384 dimensions)
//! - Automatic model file caching
//! - Batch embedding for efficiency
//!
//! ## Requirements
//! - FR-01: Local embedding via Candle
//! - No external API dependencies
//! - Works offline after initial model download

pub mod cache;
pub mod candle;
pub mod error;
pub mod model;

pub use crate::candle::CandleEmbedder;
pub use cache::{get_or_download_model, ModelCache, ModelPaths, DEFAULT_MODEL_REPO, MODEL_FILES};
pub use error::EmbeddingError;
pub use model::{Embedding, EmbeddingModel, ModelInfo};
