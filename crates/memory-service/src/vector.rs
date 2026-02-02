//! VectorTeleport RPC implementation.
//!
//! Provides semantic similarity search over TOC nodes and grips
//! using HNSW vector index.

use std::sync::Arc;

use tonic::{Request, Response, Status};
use tracing::{debug, info};

use memory_embeddings::{CandleEmbedder, EmbeddingModel};
use memory_vector::{DocType, HnswIndex, VectorIndex, VectorMetadata};

use crate::pb::{
    GetVectorIndexStatusRequest, VectorIndexStatus, VectorMatch, VectorTargetType,
    VectorTeleportRequest, VectorTeleportResponse,
};

/// Handler for vector search operations.
pub struct VectorTeleportHandler {
    embedder: Arc<CandleEmbedder>,
    index: Arc<std::sync::RwLock<HnswIndex>>,
    metadata: Arc<VectorMetadata>,
}

impl VectorTeleportHandler {
    /// Create a new vector teleport handler.
    pub fn new(
        embedder: Arc<CandleEmbedder>,
        index: Arc<std::sync::RwLock<HnswIndex>>,
        metadata: Arc<VectorMetadata>,
    ) -> Self {
        Self {
            embedder,
            index,
            metadata,
        }
    }

    /// Check if the vector index is available for search.
    pub fn is_available(&self) -> bool {
        let index = self.index.read().unwrap();
        index.len() > 0
    }

    /// Get the current vector index status.
    pub fn get_status(&self) -> VectorIndexStatus {
        let index = self.index.read().unwrap();
        let stats = index.stats();
        VectorIndexStatus {
            available: stats.available && stats.vector_count > 0,
            vector_count: stats.vector_count as i64,
            dimension: stats.dimension as i32,
            last_indexed: String::new(),
            index_path: index.index_file().to_string_lossy().to_string(),
            size_bytes: stats.size_bytes as i64,
        }
    }

    /// Handle VectorTeleport RPC request.
    pub async fn vector_teleport(
        &self,
        request: Request<VectorTeleportRequest>,
    ) -> Result<Response<VectorTeleportResponse>, Status> {
        let req = request.into_inner();
        let query = &req.query;
        let top_k = if req.top_k > 0 {
            req.top_k as usize
        } else {
            10
        };
        let min_score = req.min_score;

        debug!(query = %query, top_k = top_k, "VectorTeleport request");

        let status = self.get_status();
        if !status.available {
            return Ok(Response::new(VectorTeleportResponse {
                matches: vec![],
                index_status: Some(status),
            }));
        }

        // Embed query using spawn_blocking for CPU-bound work
        let embedder = self.embedder.clone();
        let query_owned = query.to_string();
        let embedding = tokio::task::spawn_blocking(move || embedder.embed(&query_owned))
            .await
            .map_err(|e| Status::internal(format!("Task error: {}", e)))?
            .map_err(|e| Status::internal(format!("Embedding failed: {}", e)))?;

        // Search index
        let results = {
            let index = self.index.read().unwrap();
            index
                .search(&embedding, top_k)
                .map_err(|e| Status::internal(format!("Search failed: {}", e)))?
        };

        // Convert to matches with metadata lookup
        let mut matches = Vec::new();
        for result in results {
            if result.score < min_score {
                continue;
            }

            if let Ok(Some(entry)) = self.metadata.get(result.vector_id) {
                // Target type filter
                if !self.matches_target(req.target, entry.doc_type) {
                    continue;
                }

                // Time filter
                if let Some(ref tf) = req.time_filter {
                    if entry.created_at < tf.start_ms || entry.created_at >= tf.end_ms {
                        continue;
                    }
                }

                matches.push(VectorMatch {
                    doc_id: entry.doc_id,
                    doc_type: entry.doc_type.as_str().to_string(),
                    score: result.score,
                    text_preview: entry.text_preview,
                    timestamp_ms: entry.created_at,
                });
            }
        }

        info!(query = %query, results = matches.len(), "VectorTeleport complete");

        Ok(Response::new(VectorTeleportResponse {
            matches,
            index_status: Some(status),
        }))
    }

    /// Handle GetVectorIndexStatus RPC request.
    pub async fn get_vector_index_status(
        &self,
        _request: Request<GetVectorIndexStatusRequest>,
    ) -> Result<Response<VectorIndexStatus>, Status> {
        Ok(Response::new(self.get_status()))
    }

    /// Check if a document type matches the target filter.
    fn matches_target(&self, target: i32, doc_type: DocType) -> bool {
        match VectorTargetType::try_from(target) {
            Ok(VectorTargetType::Unspecified) | Ok(VectorTargetType::All) => true,
            Ok(VectorTargetType::TocNode) => doc_type == DocType::TocNode,
            Ok(VectorTargetType::Grip) => doc_type == DocType::Grip,
            Err(_) => true,
        }
    }
}

#[cfg(test)]
mod tests {
    // Integration tests require embedding model download
    // Run with: cargo test -p memory-service --features integration -- --ignored
}
