# Topic Graph Memory - Technical Plan

## Overview

This phase adds semantic topic extraction and navigation to the agent-memory system. It enables conceptual discovery of recurring themes across conversations, with time-decayed importance scoring and topic relationships.

**Phase Type:** Future phase (not yet numbered)
**Depends On:** Phase 12 (Vector Teleport - for embeddings)
**Required By:** None (optional enhancement)

## Goals

1. Extract recurring topics from TOC summaries via embedding clustering
2. Generate human-readable topic labels via LLM or keyword extraction
3. Track topic importance with time-decayed scoring
4. Discover topic relationships (similarity, hierarchy)
5. Enable topic-based navigation alongside existing search
6. **Fully optional**: Users can disable topics entirely via configuration

---

## Optional Feature: User Opt-Out

Topic Graph Memory is **completely optional**. Users may choose to disable it for:
- **Resource constraints**: Topic extraction requires embeddings and LLM calls
- **Simplicity preference**: BM25 + vector search may be sufficient
- **Privacy concerns**: Some users prefer no additional analysis

### Configuration for Opt-Out

```toml
# ~/.config/agent-memory/config.toml

[topics]
enabled = false  # Disables all topic functionality
```

When disabled:
- No topic extraction runs
- No CF_TOPICS data created
- Topic RPCs return `UNAVAILABLE` status
- BM25, vector, and TOC navigation work normally

### Agent Skill Awareness

Agent skills **MUST** check configuration before using topic features:

```markdown
## Before Using Topics

1. Check if topics are available:
   - Call `GetTopicGraphStatus` RPC
   - If `enabled: false` or `healthy: false`, use alternatives

2. When topics are disabled:
   - Use Vector Teleport (Phase 12) for semantic search
   - Use BM25 Teleport (Phase 11) for keyword search
   - Use Agentic TOC Search (Phase 10.5) as fallback

3. When topics are enabled:
   - Use GetTopTopics to surface important themes
   - Use GetRelatedTopics for conceptual exploration
   - Use GetTocNodesForTopic to drill into specifics
```

---

## Data Architecture

### What Gets Indexed

Topics are extracted from TOC node summaries at day level and above:

```
+------------------------------------------------------------------+
|  YEAR (toc:year:2026)                                            |
|  +-- Summary: "2026 focused on authentication and API design"    |
|  +-- Topics: [Authentication, API Design] (broad themes)         |
+------------------------------------------------------------------+
|  MONTH (toc:month:2026-01)                                       |
|  +-- Summary: "January: OAuth2 implementation, bug fixes"        |
|  +-- Topics: [OAuth2, Bug Fixes, Security]                       |
+------------------------------------------------------------------+
|  WEEK (toc:week:2026-W04)                                        |
|  +-- Summary: "Week 4: JWT debugging, token refresh"             |
|  +-- Topics: [JWT Tokens, Token Refresh]                         |
+------------------------------------------------------------------+
|  DAY (toc:day:2026-01-30)                                        |
|  +-- Summary: "Resolved JWT expiry bug, added refresh logic"     |
|  +-- Topics: [JWT Tokens, Bug Fixes]                             |
+------------------------------------------------------------------+
|  SEGMENT (toc:segment:2026-01-30:abc123) [LEAF NODE]             |
|  +-- NOT directly indexed for topics                             |
|  +-- Topics inferred from parent day node                        |
+------------------------------------------------------------------+
```

### Storage Architecture

```
RocksDB (Source of Truth)
+-- CF_TOC_NODES     # TOC nodes (existing)
+-- CF_GRIPS         # Grips (existing)
+-- CF_EVENTS        # Events (existing)
+-- CF_EMBEDDINGS    # Embeddings (Phase 12)
+-- CF_TOPICS        # NEW: Topic records
    +-- topic:{topic_id} -> Topic struct
+-- CF_TOPIC_LINKS   # NEW: Topic to node associations
    +-- link:{topic_id}:{node_id} -> TopicLink struct
    +-- node:{node_id}:{topic_id} -> (reverse index)
+-- CF_TOPIC_RELS    # NEW: Topic relationships
    +-- rel:{from_id}:{to_id} -> TopicRelationship struct

Key Insight: Topics are an overlay on top of existing TOC structure.
If corrupted or deleted, they can be rebuilt from TOC summaries.
```

### Topic Extraction Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│                    Topic Extraction Pipeline                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Step 1: Collect Summaries                                       │
│  ┌──────────────────┐                                           │
│  │ Load TOC nodes   │ ─► Day, Week, Month summaries             │
│  │ (day+ level)     │                                           │
│  └────────┬─────────┘                                           │
│           │                                                      │
│           ▼                                                      │
│  Step 2: Generate Embeddings                                     │
│  ┌──────────────────┐                                           │
│  │ Embed summaries  │ ─► Uses Phase 12 embedding model          │
│  │ (batch)          │                                           │
│  └────────┬─────────┘                                           │
│           │                                                      │
│           ▼                                                      │
│  Step 3: Cluster Embeddings                                      │
│  ┌──────────────────┐                                           │
│  │ HDBSCAN or       │ ─► Groups similar summaries               │
│  │ K-means          │    into topic clusters                    │
│  └────────┬─────────┘                                           │
│           │                                                      │
│           ▼                                                      │
│  Step 4: Label Topics                                            │
│  ┌──────────────────┐                                           │
│  │ LLM or keyword   │ ─► Generate human-readable label          │
│  │ extraction       │    for each cluster                       │
│  └────────┬─────────┘                                           │
│           │                                                      │
│           ▼                                                      │
│  Step 5: Create Links                                            │
│  ┌──────────────────┐                                           │
│  │ Associate topics │ ─► Create TopicLink records               │
│  │ with source nodes│                                           │
│  └────────┬─────────┘                                           │
│           │                                                      │
│           ▼                                                      │
│  Step 6: Calculate Importance                                    │
│  ┌──────────────────┐                                           │
│  │ Time-decayed     │ ─► Score based on recency + frequency     │
│  │ importance       │                                           │
│  └────────┬─────────┘                                           │
│           │                                                      │
│           ▼                                                      │
│  Step 7: Discover Relationships                                  │
│  ┌──────────────────┐                                           │
│  │ Calculate topic  │ ─► Similar, parent/child relationships    │
│  │ similarities     │                                           │
│  └──────────────────┘                                           │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## gRPC API Design

### New RPCs

Add to `MemoryService` in `proto/memory.proto`:

```protobuf
// Find topics matching a query
rpc GetTopicsByQuery(GetTopicsByQueryRequest) returns (GetTopicsByQueryResponse);

// Get TOC nodes associated with a topic
rpc GetTocNodesForTopic(GetTocNodesForTopicRequest) returns (GetTocNodesForTopicResponse);

// Get most important topics
rpc GetTopTopics(GetTopTopicsRequest) returns (GetTopTopicsResponse);

// Get related topics (similar, parent, child)
rpc GetRelatedTopics(GetRelatedTopicsRequest) returns (GetRelatedTopicsResponse);

// Get topic graph health and status
rpc GetTopicGraphStatus(GetTopicGraphStatusRequest) returns (GetTopicGraphStatusResponse);
```

### GetTopicsByQuery Messages

```protobuf
message GetTopicsByQueryRequest {
  // Natural language query (will be embedded)
  string query = 1;
  // Maximum topics to return
  int32 limit = 2;
  // Minimum similarity score (0.0-1.0)
  float min_score = 3;
}

message GetTopicsByQueryResponse {
  // Matching topics sorted by similarity
  repeated Topic topics = 1;
  // Corresponding similarity scores
  repeated float scores = 2;
}
```

### GetTocNodesForTopic Messages

```protobuf
message GetTocNodesForTopicRequest {
  // Topic ID to get nodes for
  string topic_id = 1;
  // Maximum nodes to return
  int32 limit = 2;
  // Minimum relevance score
  float min_relevance = 3;
}

message TopicNodeLink {
  string node_id = 1;
  string title = 2;
  TocLevel level = 3;
  float relevance = 4;
  int64 timestamp_ms = 5;
}

message GetTocNodesForTopicResponse {
  repeated TopicNodeLink nodes = 1;
  bool has_more = 2;
}
```

### GetTopTopics Messages

```protobuf
message GetTopTopicsRequest {
  // Maximum topics to return
  int32 limit = 1;
  // Optional time range filter
  optional int64 start_time_ms = 2;
  optional int64 end_time_ms = 3;
}

message GetTopTopicsResponse {
  repeated Topic topics = 1;
}
```

### GetRelatedTopics Messages

```protobuf
message GetRelatedTopicsRequest {
  // Topic to find relationships for
  string topic_id = 1;
  // Filter by relationship types (empty = all)
  repeated RelationshipType relationship_types = 2;
  // Maximum related topics to return
  int32 limit = 3;
}

message RelatedTopic {
  Topic topic = 1;
  RelationshipType relationship_type = 2;
  float score = 3;
}

message GetRelatedTopicsResponse {
  repeated RelatedTopic related = 1;
}
```

### GetTopicGraphStatus Messages

```protobuf
message GetTopicGraphStatusRequest {}

message GetTopicGraphStatusResponse {
  bool enabled = 1;
  bool healthy = 2;
  int64 topic_count = 3;
  int64 link_count = 4;
  int64 last_extraction_ms = 5;
  string message = 6;
}
```

---

## Implementation Components

### Component Layout

| Component | Crate | File | Purpose |
|-----------|-------|------|---------|
| Topic storage | `memory-topics` (new) | `src/storage.rs` | CF_TOPICS operations |
| Topic extraction | `memory-topics` | `src/extraction.rs` | Clustering logic |
| Topic labeling | `memory-topics` | `src/labeling.rs` | LLM/keyword labeling |
| Importance scoring | `memory-topics` | `src/importance.rs` | Time-decay calculation |
| Relationships | `memory-topics` | `src/relationships.rs` | Similarity and hierarchy |
| Topic service | `memory-service` | `src/topic_service.rs` | RPC handlers |
| Proto definitions | `proto` | `memory.proto` | Message definitions |
| CLI commands | `memory-daemon` | `src/cli.rs` | `topics` subcommand |
| Config | `memory-core` | `src/config.rs` | Topics configuration |

### Topic Extraction Implementation

```rust
// memory-topics/src/extraction.rs

use crate::{Topic, TopicLink};

/// Configuration for topic extraction
pub struct ExtractionConfig {
    /// Minimum cluster size to create a topic
    pub min_cluster_size: usize,
    /// Cosine similarity threshold for clustering
    pub similarity_threshold: f32,
    /// Batch size for processing
    pub batch_size: usize,
}

/// Topic extractor using embedding clustering
pub struct TopicExtractor {
    config: ExtractionConfig,
    embedding_model: Arc<EmbeddingModel>,
}

impl TopicExtractor {
    /// Extract topics from TOC nodes
    pub async fn extract_topics(
        &self,
        nodes: &[TocNode],
    ) -> Result<Vec<(Topic, Vec<TopicLink>)>, Error> {
        // Step 1: Generate embeddings for node summaries
        let summaries: Vec<&str> = nodes
            .iter()
            .filter(|n| matches!(n.level, TocLevel::Day | TocLevel::Week | TocLevel::Month))
            .map(|n| n.summary.as_str())
            .collect();

        let embeddings = self.embedding_model.embed_batch(&summaries).await?;

        // Step 2: Cluster embeddings
        let clusters = self.cluster_embeddings(&embeddings)?;

        // Step 3: Create topics from clusters
        let mut results = Vec::new();
        for cluster in clusters {
            if cluster.member_indices.len() < self.config.min_cluster_size {
                continue;
            }

            // Calculate cluster centroid
            let centroid = self.calculate_centroid(&embeddings, &cluster.member_indices);

            // Create topic
            let topic = Topic {
                topic_id: ulid::Ulid::new().to_string(),
                label: String::new(), // Filled by labeling step
                embedding: centroid,
                importance_score: 0.0, // Calculated separately
                node_count: cluster.member_indices.len() as u32,
                created_at: Utc::now(),
                last_mentioned_at: Utc::now(),
                status: TopicStatus::Active,
                keywords: Vec::new(),
            };

            // Create links to source nodes
            let links: Vec<TopicLink> = cluster.member_indices
                .iter()
                .map(|&idx| TopicLink {
                    topic_id: topic.topic_id.clone(),
                    node_id: nodes[idx].node_id.clone(),
                    relevance: cluster.similarities[idx],
                    created_at: Utc::now(),
                })
                .collect();

            results.push((topic, links));
        }

        Ok(results)
    }

    /// Cluster embeddings using HDBSCAN-like algorithm
    fn cluster_embeddings(&self, embeddings: &[Vec<f32>]) -> Result<Vec<Cluster>, Error> {
        // Simple agglomerative clustering for MVP
        // Can upgrade to HDBSCAN for better quality
        let mut clusters = Vec::new();
        let mut assigned = vec![false; embeddings.len()];

        for i in 0..embeddings.len() {
            if assigned[i] {
                continue;
            }

            let mut cluster = Cluster {
                member_indices: vec![i],
                similarities: vec![1.0],
            };

            for j in (i + 1)..embeddings.len() {
                if assigned[j] {
                    continue;
                }

                let similarity = cosine_similarity(&embeddings[i], &embeddings[j]);
                if similarity >= self.config.similarity_threshold {
                    cluster.member_indices.push(j);
                    cluster.similarities.push(similarity);
                    assigned[j] = true;
                }
            }

            if cluster.member_indices.len() >= self.config.min_cluster_size {
                assigned[i] = true;
                clusters.push(cluster);
            }
        }

        Ok(clusters)
    }

    /// Calculate centroid embedding for a cluster
    fn calculate_centroid(&self, embeddings: &[Vec<f32>], indices: &[usize]) -> Vec<f32> {
        let dim = embeddings[0].len();
        let mut centroid = vec![0.0; dim];

        for &idx in indices {
            for (i, val) in embeddings[idx].iter().enumerate() {
                centroid[i] += val;
            }
        }

        let n = indices.len() as f32;
        for val in centroid.iter_mut() {
            *val /= n;
        }

        // Normalize
        let norm: f32 = centroid.iter().map(|x| x * x).sum::<f32>().sqrt();
        for val in centroid.iter_mut() {
            *val /= norm;
        }

        centroid
    }
}

struct Cluster {
    member_indices: Vec<usize>,
    similarities: Vec<f32>,
}
```

### Topic Labeling Implementation

```rust
// memory-topics/src/labeling.rs

use crate::Topic;

/// Configuration for topic labeling
pub struct LabelingConfig {
    /// Use LLM for labeling
    pub use_llm: bool,
    /// Fall back to keywords if LLM fails
    pub fallback_to_keywords: bool,
    /// Maximum label length
    pub max_label_length: usize,
}

/// Topic labeler using LLM or keywords
pub struct TopicLabeler {
    config: LabelingConfig,
    summarizer: Option<Arc<dyn Summarizer>>,
}

impl TopicLabeler {
    /// Generate label for a topic based on its linked nodes
    pub async fn label_topic(
        &self,
        topic: &mut Topic,
        linked_summaries: &[String],
    ) -> Result<(), Error> {
        if self.config.use_llm {
            if let Some(ref summarizer) = self.summarizer {
                match self.label_with_llm(summarizer, linked_summaries).await {
                    Ok(label) => {
                        topic.label = self.truncate_label(&label);
                        topic.keywords = self.extract_keywords(linked_summaries);
                        return Ok(());
                    }
                    Err(e) => {
                        tracing::warn!("LLM labeling failed: {}, falling back", e);
                    }
                }
            }
        }

        // Fallback to keyword extraction
        if self.config.fallback_to_keywords {
            topic.keywords = self.extract_keywords(linked_summaries);
            topic.label = self.label_from_keywords(&topic.keywords);
        }

        Ok(())
    }

    /// Generate label using LLM
    async fn label_with_llm(
        &self,
        summarizer: &dyn Summarizer,
        summaries: &[String],
    ) -> Result<String, Error> {
        let prompt = format!(
            "Generate a concise 2-4 word topic label for these related discussions:\n\n{}",
            summaries.join("\n\n")
        );

        let response = summarizer.summarize(&prompt).await?;

        // Extract just the label from response
        let label = response.lines().next().unwrap_or(&response).trim();
        Ok(label.to_string())
    }

    /// Extract keywords from summaries using TF-IDF-like scoring
    fn extract_keywords(&self, summaries: &[String]) -> Vec<String> {
        use std::collections::HashMap;

        let mut word_counts: HashMap<String, usize> = HashMap::new();
        let stopwords = self.get_stopwords();

        for summary in summaries {
            for word in summary.split_whitespace() {
                let normalized = word.to_lowercase()
                    .trim_matches(|c: char| !c.is_alphanumeric())
                    .to_string();

                if normalized.len() >= 3 && !stopwords.contains(&normalized.as_str()) {
                    *word_counts.entry(normalized).or_insert(0) += 1;
                }
            }
        }

        let mut keywords: Vec<_> = word_counts.into_iter().collect();
        keywords.sort_by(|a, b| b.1.cmp(&a.1));

        keywords.into_iter()
            .take(5)
            .map(|(word, _)| word)
            .collect()
    }

    /// Generate label from top keywords
    fn label_from_keywords(&self, keywords: &[String]) -> String {
        keywords.iter()
            .take(3)
            .map(|k| {
                let mut chars = k.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().chain(chars).collect(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn truncate_label(&self, label: &str) -> String {
        if label.len() <= self.config.max_label_length {
            label.to_string()
        } else {
            format!("{}...", &label[..self.config.max_label_length - 3])
        }
    }

    fn get_stopwords(&self) -> std::collections::HashSet<&'static str> {
        ["the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for",
         "of", "with", "by", "from", "as", "is", "was", "are", "were", "been",
         "be", "have", "has", "had", "do", "does", "did", "will", "would",
         "could", "should", "may", "might", "must", "this", "that", "these",
         "those", "it", "its", "we", "our", "you", "your"]
            .into_iter().collect()
    }
}
```

### Importance Scoring Implementation

```rust
// memory-topics/src/importance.rs

/// Configuration for importance scoring
pub struct ImportanceConfig {
    /// Half-life for importance decay (days)
    pub half_life_days: u32,
    /// Boost for recent mentions (within 7 days)
    pub recency_boost: f64,
}

/// Calculator for time-decayed importance scores
pub struct ImportanceCalculator {
    config: ImportanceConfig,
}

impl ImportanceCalculator {
    /// Calculate importance score for a topic based on its mentions
    pub fn calculate_importance(
        &self,
        mention_timestamps: &[DateTime<Utc>],
        now: DateTime<Utc>,
    ) -> f64 {
        let mut score = 0.0;
        let half_life_secs = self.config.half_life_days as f64 * 24.0 * 3600.0;

        for ts in mention_timestamps {
            let age_secs = (now - *ts).num_seconds() as f64;
            let days_ago = age_secs / (24.0 * 3600.0);

            // Base weight
            let mut weight = 1.0;

            // Apply recency boost for recent mentions
            if days_ago <= 7.0 {
                weight *= self.config.recency_boost;
            }

            // Apply time decay
            let decay_factor = 0.5_f64.powf(age_secs / half_life_secs);
            score += weight * decay_factor;
        }

        score
    }

    /// Recalculate importance for all topics
    pub async fn recalculate_all(
        &self,
        storage: &TopicStorage,
    ) -> Result<Vec<(String, f64)>, Error> {
        let topics = storage.list_all_topics().await?;
        let now = Utc::now();
        let mut updates = Vec::new();

        for topic in topics {
            let links = storage.get_links_for_topic(&topic.topic_id).await?;
            let timestamps: Vec<_> = links.iter()
                .map(|l| l.created_at)
                .collect();

            let new_score = self.calculate_importance(&timestamps, now);
            updates.push((topic.topic_id, new_score));
        }

        Ok(updates)
    }
}
```

### gRPC Service Implementation

```rust
// memory-service/src/topic_service.rs

impl MemoryService {
    pub async fn get_topics_by_query(
        &self,
        request: Request<GetTopicsByQueryRequest>,
    ) -> Result<Response<GetTopicsByQueryResponse>, Status> {
        let req = request.into_inner();

        // Check if topics are enabled
        let topic_storage = self.topic_storage.as_ref()
            .ok_or_else(|| Status::unavailable("Topic graph not enabled"))?;

        // Embed the query
        let query_embedding = self.embedding_model
            .embed(&req.query)
            .await
            .map_err(|e| Status::internal(format!("Embedding failed: {}", e)))?;

        // Find similar topics
        let all_topics = topic_storage.list_active_topics().await
            .map_err(|e| Status::internal(e.to_string()))?;

        let mut scored_topics: Vec<(Topic, f32)> = all_topics
            .into_iter()
            .map(|t| {
                let similarity = cosine_similarity(&query_embedding, &t.embedding);
                (t, similarity)
            })
            .filter(|(_, score)| *score >= req.min_score)
            .collect();

        scored_topics.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        scored_topics.truncate(req.limit as usize);

        let topics: Vec<Topic> = scored_topics.iter().map(|(t, _)| t.clone()).collect();
        let scores: Vec<f32> = scored_topics.iter().map(|(_, s)| *s).collect();

        Ok(Response::new(GetTopicsByQueryResponse { topics, scores }))
    }

    pub async fn get_toc_nodes_for_topic(
        &self,
        request: Request<GetTocNodesForTopicRequest>,
    ) -> Result<Response<GetTocNodesForTopicResponse>, Status> {
        let req = request.into_inner();

        let topic_storage = self.topic_storage.as_ref()
            .ok_or_else(|| Status::unavailable("Topic graph not enabled"))?;

        // Get links for this topic
        let links = topic_storage.get_links_for_topic(&req.topic_id).await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Filter by relevance and sort
        let mut filtered: Vec<_> = links
            .into_iter()
            .filter(|l| l.relevance >= req.min_relevance)
            .collect();

        filtered.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap_or(Ordering::Equal));

        let limit = req.limit as usize;
        let has_more = filtered.len() > limit;
        filtered.truncate(limit);

        // Enrich with node details
        let mut nodes = Vec::with_capacity(filtered.len());
        for link in filtered {
            if let Ok(Some(node)) = self.storage.get_toc_node(&link.node_id).await {
                nodes.push(TopicNodeLink {
                    node_id: link.node_id,
                    title: node.title,
                    level: node.level as i32,
                    relevance: link.relevance,
                    timestamp_ms: node.start_time.timestamp_millis(),
                });
            }
        }

        Ok(Response::new(GetTocNodesForTopicResponse { nodes, has_more }))
    }

    pub async fn get_top_topics(
        &self,
        request: Request<GetTopTopicsRequest>,
    ) -> Result<Response<GetTopTopicsResponse>, Status> {
        let req = request.into_inner();

        let topic_storage = self.topic_storage.as_ref()
            .ok_or_else(|| Status::unavailable("Topic graph not enabled"))?;

        let mut topics = topic_storage.list_active_topics().await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Filter by time range if specified
        if let Some(start_ms) = req.start_time_ms {
            let start = DateTime::from_timestamp_millis(start_ms).unwrap_or_default();
            topics.retain(|t| t.last_mentioned_at >= start);
        }
        if let Some(end_ms) = req.end_time_ms {
            let end = DateTime::from_timestamp_millis(end_ms).unwrap_or_default();
            topics.retain(|t| t.last_mentioned_at <= end);
        }

        // Sort by importance
        topics.sort_by(|a, b| {
            b.importance_score.partial_cmp(&a.importance_score).unwrap_or(Ordering::Equal)
        });

        topics.truncate(req.limit as usize);

        Ok(Response::new(GetTopTopicsResponse { topics }))
    }

    pub async fn get_related_topics(
        &self,
        request: Request<GetRelatedTopicsRequest>,
    ) -> Result<Response<GetRelatedTopicsResponse>, Status> {
        let req = request.into_inner();

        let topic_storage = self.topic_storage.as_ref()
            .ok_or_else(|| Status::unavailable("Topic graph not enabled"))?;

        let relationships = topic_storage
            .get_relationships_for_topic(&req.topic_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Filter by relationship type if specified
        let filtered: Vec<_> = if req.relationship_types.is_empty() {
            relationships
        } else {
            relationships.into_iter()
                .filter(|r| req.relationship_types.contains(&(r.relationship_type as i32)))
                .collect()
        };

        // Get full topic details and build response
        let mut related = Vec::new();
        for rel in filtered.into_iter().take(req.limit as usize) {
            if let Ok(Some(topic)) = topic_storage.get_topic(&rel.to_topic_id).await {
                related.push(RelatedTopic {
                    topic: Some(topic),
                    relationship_type: rel.relationship_type as i32,
                    score: rel.score,
                });
            }
        }

        Ok(Response::new(GetRelatedTopicsResponse { related }))
    }

    pub async fn get_topic_graph_status(
        &self,
        _request: Request<GetTopicGraphStatusRequest>,
    ) -> Result<Response<GetTopicGraphStatusResponse>, Status> {
        let (enabled, healthy, stats) = match &self.topic_storage {
            Some(storage) => {
                let stats = storage.get_stats().await.unwrap_or_default();
                (true, stats.topic_count > 0, stats)
            }
            None => (false, false, TopicStats::default()),
        };

        Ok(Response::new(GetTopicGraphStatusResponse {
            enabled,
            healthy,
            topic_count: stats.topic_count as i64,
            link_count: stats.link_count as i64,
            last_extraction_ms: stats.last_extraction_ms,
            message: if enabled {
                format!("{} topics indexed", stats.topic_count)
            } else {
                "Topic graph is disabled".to_string()
            },
        }))
    }
}
```

---

## Configuration

### Config Schema

```toml
# ~/.config/agent-memory/config.toml

[topics]
# Master switch for topic functionality
# Default: false (opt-in)
enabled = false

[topics.extraction]
# Enable automatic extraction
enabled = true
# Minimum nodes to form a topic cluster
min_cluster_size = 3
# Similarity threshold for clustering (0.0-1.0)
similarity_threshold = 0.75
# Cron schedule for extraction
schedule = "0 4 * * *"
# Batch size for processing
batch_size = 500

[topics.labeling]
# Use LLM for labels
enabled = true
# Model (default = use configured summarizer)
model = "default"
# Fall back to keywords if LLM unavailable
fallback_to_keywords = true
# Max label length
max_label_length = 50

[topics.importance]
# Half-life for decay (days)
half_life_days = 30
# Boost for recent mentions
recency_boost = 2.0
# Minimum score to stay active
min_active_score = 0.1

[topics.relationships]
# Enable relationship discovery
enabled = true
# Similarity threshold for "similar" relationship
similarity_threshold = 0.6
# Enable parent/child inference
hierarchy_enabled = true
# Max hierarchy depth
max_hierarchy_depth = 3

[topics.lifecycle]
# Enable pruning
enabled = true
# Days before pruning inactive topics
prune_after_days = 180
# Allow resurrection
resurrection_enabled = true
# Prune schedule
prune_schedule = "0 5 * * 0"
```

---

## CLI Commands

### New Commands

```bash
# List top topics by importance
memory-daemon topics list --limit 10

# Search for topics
memory-daemon topics search "authentication"

# Show topic details
memory-daemon topics show 01HQXYZ123ABC

# Show related topics
memory-daemon topics related 01HQXYZ123ABC --type similar

# Show nodes linked to topic
memory-daemon topics nodes 01HQXYZ123ABC --limit 5

# Show topic graph status
memory-daemon topics status

# Force topic extraction (admin)
memory-daemon topics extract --force
```

### CLI Implementation

```rust
// memory-daemon/src/cli.rs

#[derive(Parser)]
enum Commands {
    // ... existing commands ...

    /// Topic graph operations
    Topics {
        #[command(subcommand)]
        action: TopicAction,
    },
}

#[derive(Subcommand)]
enum TopicAction {
    /// List top topics by importance
    List {
        #[arg(long, default_value = "10")]
        limit: u32,
    },

    /// Search for topics by query
    Search {
        query: String,
        #[arg(long, default_value = "10")]
        limit: u32,
    },

    /// Show topic details
    Show {
        topic_id: String,
    },

    /// Show related topics
    Related {
        topic_id: String,
        #[arg(long)]
        r#type: Option<String>,
        #[arg(long, default_value = "10")]
        limit: u32,
    },

    /// Show nodes linked to topic
    Nodes {
        topic_id: String,
        #[arg(long, default_value = "10")]
        limit: u32,
    },

    /// Show topic graph status
    Status,

    /// Force topic extraction (admin)
    Extract {
        #[arg(long)]
        force: bool,
    },
}
```

---

## Testing Strategy

### Unit Tests

```rust
// memory-topics/src/extraction.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_similar_summaries() {
        let extractor = TopicExtractor::new(ExtractionConfig {
            min_cluster_size: 2,
            similarity_threshold: 0.7,
            batch_size: 100,
        });

        // Mock embeddings (similar pairs)
        let embeddings = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.95, 0.05, 0.0],  // Similar to first
            vec![0.0, 1.0, 0.0],    // Different
        ];

        let clusters = extractor.cluster_embeddings(&embeddings).unwrap();

        assert_eq!(clusters.len(), 1);
        assert!(clusters[0].member_indices.contains(&0));
        assert!(clusters[0].member_indices.contains(&1));
    }

    #[test]
    fn test_centroid_calculation() {
        let extractor = TopicExtractor::default();

        let embeddings = vec![
            vec![1.0, 0.0],
            vec![0.0, 1.0],
        ];

        let centroid = extractor.calculate_centroid(&embeddings, &[0, 1]);

        // Should be normalized [0.5, 0.5] -> [0.707, 0.707]
        assert!((centroid[0] - 0.707).abs() < 0.01);
        assert!((centroid[1] - 0.707).abs() < 0.01);
    }
}

// memory-topics/src/importance.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_importance_decay() {
        let calc = ImportanceCalculator::new(ImportanceConfig {
            half_life_days: 30,
            recency_boost: 1.0,
        });

        let now = Utc::now();
        let mentions = vec![
            now,
            now - Duration::days(30),
            now - Duration::days(60),
        ];

        let score = calc.calculate_importance(&mentions, now);

        // Today: 1.0, 30 days: 0.5, 60 days: 0.25
        assert!((score - 1.75).abs() < 0.01);
    }

    #[test]
    fn test_recency_boost() {
        let calc = ImportanceCalculator::new(ImportanceConfig {
            half_life_days: 30,
            recency_boost: 2.0,
        });

        let now = Utc::now();
        let recent = vec![now - Duration::days(3)];
        let old = vec![now - Duration::days(30)];

        let recent_score = calc.calculate_importance(&recent, now);
        let old_score = calc.calculate_importance(&old, now);

        // Recent should have 2x boost, then decay
        // Old should have no boost, half decay
        assert!(recent_score > old_score * 2.0);
    }
}
```

### Integration Tests

```rust
// memory-service/tests/topic_integration.rs

#[tokio::test]
async fn test_topic_extraction_and_query() {
    let server = setup_test_server_with_topics().await;

    // Ingest test conversations
    ingest_event(&server, "Discussed JWT token authentication").await;
    ingest_event(&server, "Fixed OAuth2 login flow").await;
    ingest_event(&server, "Debugged token refresh mechanism").await;
    ingest_event(&server, "Database migration completed").await;

    // Trigger TOC build
    trigger_toc_build(&server).await;

    // Trigger topic extraction
    trigger_topic_extraction(&server).await;

    // Query for auth-related topics
    let response = server.get_topics_by_query(GetTopicsByQueryRequest {
        query: "authentication".to_string(),
        limit: 5,
        min_score: 0.5,
    }).await.unwrap();

    // Should find an auth-related topic
    assert!(!response.topics.is_empty());
    let topic = &response.topics[0];
    assert!(
        topic.label.to_lowercase().contains("auth") ||
        topic.keywords.iter().any(|k| k.contains("auth") || k.contains("token"))
    );
}

#[tokio::test]
async fn test_topic_relationships() {
    let server = setup_test_server_with_topics().await;

    // Create topics with relationships
    create_test_topics(&server, &[
        ("Authentication", vec!["JWT", "OAuth", "login"]),
        ("Security", vec!["auth", "encryption", "OWASP"]),
    ]).await;

    // Get related topics
    let auth_topic = find_topic_by_label(&server, "Authentication").await;
    let response = server.get_related_topics(GetRelatedTopicsRequest {
        topic_id: auth_topic.topic_id,
        relationship_types: vec![],
        limit: 10,
    }).await.unwrap();

    // Should find Security as related
    assert!(response.related.iter().any(|r| {
        r.topic.as_ref().map(|t| t.label.contains("Security")).unwrap_or(false)
    }));
}
```

---

## Success Criteria

| Criterion | Verification |
|-----------|--------------|
| Topics extracted from TOC summaries | Integration test with mock data |
| Topic labels are meaningful | Manual review of generated labels |
| Importance scoring reflects recency | Unit test with time-decayed scores |
| Similar topics are related | Integration test with semantic pairs |
| Topic queries find relevant topics | Integration test with query matching |
| Fallback when disabled | Status check returns disabled |
| CLI commands work | Manual testing |

---

## Implementation Plan

### Wave 1: Topic Extraction (Plan TOPIC-01)

**Files Modified:**
- `Cargo.toml` - Add memory-topics crate
- `crates/memory-topics/src/lib.rs` (new) - Crate entry
- `crates/memory-topics/src/storage.rs` (new) - CF_TOPICS operations
- `crates/memory-topics/src/extraction.rs` (new) - Clustering logic
- `crates/memory-core/src/config.rs` - Topic config

**Tasks:**
1. Create memory-topics crate
2. Implement CF_TOPICS storage
3. Implement embedding clustering
4. Add Topic struct and serialization
5. Unit tests for extraction

### Wave 2: Topic Labeling (Plan TOPIC-02)

**Files Modified:**
- `crates/memory-topics/src/labeling.rs` (new) - LLM/keyword labeling
- `crates/memory-topics/src/lib.rs` - Wire up labeling

**Tasks:**
1. Implement LLM-based labeling
2. Implement keyword extraction fallback
3. Add label truncation and uniqueness
4. Unit tests for labeling

### Wave 3: Importance Scoring (Plan TOPIC-03)

**Files Modified:**
- `crates/memory-topics/src/importance.rs` (new) - Time decay
- `proto/memory.proto` - GetTopTopics RPC
- `crates/memory-service/src/topic_service.rs` (new) - Handler

**Tasks:**
1. Implement time-decay calculation
2. Add GetTopTopics RPC
3. Add scheduled recalculation job
4. Unit and integration tests

### Wave 4: Topic Relationships (Plan TOPIC-04)

**Files Modified:**
- `crates/memory-topics/src/relationships.rs` (new)
- `crates/memory-topics/src/storage.rs` - CF_TOPIC_RELS
- `proto/memory.proto` - RelationshipType enum

**Tasks:**
1. Implement similarity relationships
2. Implement hierarchy inference
3. Add relationship storage
4. Unit tests

### Wave 5: Navigation RPCs (Plan TOPIC-05)

**Files Modified:**
- `proto/memory.proto` - All topic RPCs
- `crates/memory-service/src/topic_service.rs` - All handlers
- `crates/memory-service/src/lib.rs` - Wire up service

**Tasks:**
1. Implement GetTopicsByQuery RPC
2. Implement GetTocNodesForTopic RPC
3. Implement GetRelatedTopics RPC
4. Integration tests

### Wave 6: Lifecycle & CLI (Plan TOPIC-06)

**Files Modified:**
- `crates/memory-topics/src/lifecycle.rs` (new) - Pruning
- `crates/memory-daemon/src/cli.rs` - Topic commands
- `proto/memory.proto` - GetTopicGraphStatus RPC

**Tasks:**
1. Implement pruning logic
2. Implement resurrection logic
3. Add CLI commands
4. Add GetTopicGraphStatus RPC
5. Integration tests

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Clustering quality poor | Useless topics | Tune thresholds, use HDBSCAN |
| LLM labeling expensive | Cost concerns | Batch processing, keyword fallback |
| Too many topics | User overwhelm | Min cluster size, pruning |
| Circular relationships | Traversal issues | Validate on create, depth limits |
| Embedding model dependency | Phase 12 required | Check availability, graceful fallback |

---

## Agent Skill Integration Guide

### Checking Topic Availability

```rust
async fn use_topics_if_available(query: &str) -> Result<SearchResults> {
    let status = client.get_topic_graph_status().await?;

    if status.enabled && status.healthy {
        // Topics available - use them
        let topics = client.get_topics_by_query(GetTopicsByQueryRequest {
            query: query.to_string(),
            limit: 5,
            min_score: 0.5,
        }).await?;

        // Get nodes for top topic
        if let Some(top_topic) = topics.topics.first() {
            let nodes = client.get_toc_nodes_for_topic(GetTocNodesForTopicRequest {
                topic_id: top_topic.topic_id.clone(),
                limit: 10,
                min_relevance: 0.3,
            }).await?;

            return Ok(SearchResults::TopicBased(topics, nodes));
        }
    }

    // Fall back to vector/BM25 search
    client.hybrid_search(query).await
}
```

### Skill SKILL.md Pattern

```markdown
## Search Capability Tiers

### Tier 1: Topic-Guided (Best for exploration)
**When:** Topics enabled and healthy
**Commands:** `/memory-topics`, `/memory-explore`
**Capability:** Discovers themes and conceptual connections

### Tier 2: Semantic Search (Best for similarity)
**When:** Topics disabled, vector available
**Commands:** `/memory-search --semantic`
**Capability:** Finds similar content by meaning

### Tier 3: Keyword Search (Always reliable)
**When:** Topics and vector disabled
**Commands:** `/memory-search`
**Capability:** Finds exact keyword matches

### Tier 4: TOC Navigation (Always works)
**When:** All indexes unavailable
**Commands:** `/memory-browse`
**Capability:** Traverses time hierarchy
```

---

*Plan created: 2026-02-01*
*Target: Topic Graph Memory (Phase TBD)*
*Author: Agent Memory Team*
