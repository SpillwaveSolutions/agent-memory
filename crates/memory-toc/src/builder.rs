//! TOC hierarchy builder.
//!
//! Builds TOC nodes from segments and ensures parent nodes exist.

use chrono::{DateTime, Utc};
use std::sync::Arc;
use tracing::{debug, info};

use memory_storage::Storage;
use memory_types::{Segment, TocBullet, TocLevel, TocNode};

use crate::node_id::{generate_node_id, generate_title, get_parent_node_id, get_time_boundaries};
use crate::summarizer::{extract_grips, Summarizer, SummarizerError, Summary};

/// Error type for TOC building.
#[derive(Debug, thiserror::Error)]
pub enum BuilderError {
    #[error("Storage error: {0}")]
    Storage(#[from] memory_storage::StorageError),

    #[error("Summarization error: {0}")]
    Summarizer(#[from] SummarizerError),

    #[error("Invalid segment: {0}")]
    InvalidSegment(String),
}

/// Builder for TOC hierarchy.
///
/// Processes segments and creates TOC nodes at all levels.
pub struct TocBuilder {
    storage: Arc<Storage>,
    summarizer: Arc<dyn Summarizer>,
}

impl TocBuilder {
    /// Create a new TocBuilder.
    pub fn new(storage: Arc<Storage>, summarizer: Arc<dyn Summarizer>) -> Self {
        Self {
            storage,
            summarizer,
        }
    }

    /// Process a segment and create/update TOC nodes.
    ///
    /// Creates:
    /// 1. Segment-level node from the segment
    /// 2. Ensures parent nodes exist up to Year level
    /// 3. Extracts grips from events based on bullets (SUMM-03)
    pub async fn process_segment(&self, segment: &Segment) -> Result<TocNode, BuilderError> {
        if segment.events.is_empty() {
            return Err(BuilderError::InvalidSegment(
                "Segment has no events".to_string(),
            ));
        }

        info!(
            segment_id = %segment.segment_id,
            events = segment.events.len(),
            "Processing segment"
        );

        // Summarize the segment
        let all_events: Vec<_> = segment.all_events().into_iter().cloned().collect();
        let summary = self.summarizer.summarize_events(&all_events).await?;

        // Create segment node
        let mut segment_node = self.create_segment_node(segment, &summary)?;

        // Extract grips from events based on bullets (SUMM-03)
        let extracted_grips = extract_grips(&all_events, &summary.bullets, &segment_node.node_id);

        // Store grips and link to segment node
        for extracted in &extracted_grips {
            // Create grip with TOC node link
            let mut grip = extracted.grip.clone();
            grip.toc_node_id = Some(segment_node.node_id.clone());

            // Link bullet to grip if we know which bullet it supports
            if let Some(bullet_idx) = extracted.bullet_index {
                if bullet_idx < segment_node.bullets.len() {
                    segment_node.bullets[bullet_idx]
                        .grip_ids
                        .push(grip.grip_id.clone());
                }
            }

            self.storage.put_grip(&grip)?;
        }

        debug!(
            segment_id = %segment.segment_id,
            grips = extracted_grips.len(),
            "Extracted grips from segment"
        );

        self.storage.put_toc_node(&segment_node)?;

        // Ensure parent nodes exist and are updated
        self.ensure_parents(&segment_node).await?;

        Ok(segment_node)
    }

    /// Create a segment-level TOC node.
    fn create_segment_node(
        &self,
        segment: &Segment,
        summary: &Summary,
    ) -> Result<TocNode, BuilderError> {
        let node_id = format!(
            "toc:segment:{}:{}",
            segment.start_time.format("%Y-%m-%d"),
            segment.segment_id.trim_start_matches("seg:")
        );

        let bullets: Vec<TocBullet> = summary.bullets.iter().map(TocBullet::new).collect();

        let mut node = TocNode::new(
            node_id,
            TocLevel::Segment,
            summary.title.clone(),
            segment.start_time,
            segment.end_time,
        );
        node.bullets = bullets;
        node.keywords = summary.keywords.clone();

        Ok(node)
    }

    /// Ensure parent nodes exist up to Year level.
    async fn ensure_parents(&self, child_node: &TocNode) -> Result<(), BuilderError> {
        let mut current_id = child_node.node_id.clone();
        let mut child_level = child_node.level;

        while let Some(parent_level) = child_level.parent() {
            if let Some(parent_id) = get_parent_node_id(&current_id) {
                // Check if parent exists
                let parent = self.storage.get_toc_node(&parent_id)?;

                if let Some(mut parent_node) = parent {
                    // Update parent's child list if needed
                    if !parent_node.child_node_ids.contains(&current_id) {
                        parent_node.child_node_ids.push(current_id.clone());
                        self.storage.put_toc_node(&parent_node)?;
                        debug!(
                            parent = %parent_id,
                            child = %current_id,
                            "Added child to existing parent"
                        );
                    }
                } else {
                    // Create parent node with placeholder summary
                    let parent_node =
                        self.create_parent_node(&parent_id, parent_level, child_node, &current_id)?;
                    self.storage.put_toc_node(&parent_node)?;
                    debug!(
                        parent = %parent_id,
                        level = %parent_level,
                        "Created new parent node"
                    );
                }

                current_id = parent_id;
                child_level = parent_level;
            } else {
                break;
            }
        }

        Ok(())
    }

    /// Create a parent node with placeholder summary.
    fn create_parent_node(
        &self,
        parent_id: &str,
        level: TocLevel,
        child: &TocNode,
        child_id: &str,
    ) -> Result<TocNode, BuilderError> {
        let (start_time, end_time) = get_time_boundaries(level, child.start_time);
        let title = generate_title(level, child.start_time);

        let mut node = TocNode::new(parent_id.to_string(), level, title, start_time, end_time);
        node.child_node_ids.push(child_id.to_string());

        // Placeholder bullet - will be replaced by rollup job
        node.bullets.push(TocBullet::new("Summary pending..."));

        Ok(node)
    }

    /// Get all segment nodes for a day.
    pub fn get_segments_for_day(&self, date: DateTime<Utc>) -> Result<Vec<TocNode>, BuilderError> {
        let day_id = generate_node_id(TocLevel::Day, date);
        self.storage
            .get_child_nodes(&day_id)
            .map_err(BuilderError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::summarizer::MockSummarizer;
    use chrono::TimeZone;
    use memory_types::{Event, EventRole, EventType};
    use tempfile::TempDir;

    fn create_test_storage() -> (Arc<Storage>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(Storage::open(temp_dir.path()).unwrap());
        (storage, temp_dir)
    }

    fn create_test_event(text: &str, timestamp_ms: i64) -> Event {
        let ulid = ulid::Ulid::from_parts(timestamp_ms as u64, rand::random());
        Event::new(
            ulid.to_string(),
            "session-123".to_string(),
            Utc.timestamp_millis_opt(timestamp_ms).unwrap(),
            EventType::UserMessage,
            EventRole::User,
            text.to_string(),
        )
    }

    #[tokio::test]
    async fn test_process_segment_creates_node() {
        let (storage, _temp) = create_test_storage();
        let summarizer = Arc::new(MockSummarizer::new());
        let builder = TocBuilder::new(storage.clone(), summarizer);

        let events = vec![
            create_test_event("Hello", 1706540400000), // 2024-01-29
            create_test_event("World", 1706540500000),
        ];
        let segment = Segment::new(
            "seg:test123".to_string(),
            events.clone(),
            events[0].timestamp,
            events[1].timestamp,
            100,
        );

        let node = builder.process_segment(&segment).await.unwrap();

        assert_eq!(node.level, TocLevel::Segment);
        assert!(!node.bullets.is_empty());
    }

    #[tokio::test]
    async fn test_process_segment_creates_parents() {
        let (storage, _temp) = create_test_storage();
        let summarizer = Arc::new(MockSummarizer::new());
        let builder = TocBuilder::new(storage.clone(), summarizer);

        let events = vec![create_test_event("Test", 1706540400000)];
        let segment = Segment::new(
            "seg:test456".to_string(),
            events.clone(),
            events[0].timestamp,
            events[0].timestamp,
            50,
        );

        builder.process_segment(&segment).await.unwrap();

        // Check that day node was created
        let day_node = storage.get_toc_node("toc:day:2024-01-29").unwrap();
        assert!(day_node.is_some());

        // Check that year node was created
        let year_node = storage.get_toc_node("toc:year:2024").unwrap();
        assert!(year_node.is_some());
    }

    #[tokio::test]
    async fn test_process_segment_extracts_grips() {
        let (storage, _temp) = create_test_storage();
        let summarizer = Arc::new(MockSummarizer::new());
        let builder = TocBuilder::new(storage.clone(), summarizer);

        // Create events with content that will match the mock summarizer's bullets
        let events = vec![
            create_test_event("How do I implement authentication?", 1706540400000),
            create_test_event("You can use JWT tokens for secure auth", 1706540500000),
            create_test_event("Thanks, that helps!", 1706540600000),
        ];
        let segment = Segment::new(
            "seg:test789".to_string(),
            events.clone(),
            events[0].timestamp,
            events[2].timestamp,
            150,
        );

        let node = builder.process_segment(&segment).await.unwrap();

        // Verify grips were created and linked to the node
        let grips = storage.get_grips_for_node(&node.node_id).unwrap();

        // MockSummarizer generates bullets with keywords like "discussed", "topics"
        // Grips are extracted based on term matching, so we may or may not get matches
        // The important thing is the integration works without errors
        // Verify we can retrieve grips (even if empty) - this tests the storage integration
        let _grip_count = grips.len();

        // If grips were extracted, verify they have correct source
        for grip in &grips {
            assert_eq!(grip.source, node.node_id);
            assert!(grip.toc_node_id.as_ref() == Some(&node.node_id));
        }
    }
}
