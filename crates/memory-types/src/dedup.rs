//! In-flight dedup buffer for semantic deduplication gate.
//!
//! Provides a fixed-capacity ring buffer that stores recent event embeddings
//! and supports brute-force cosine similarity search. Used by the dedup gate
//! to detect near-duplicate events before they reach the HNSW index.

/// A single entry in the in-flight dedup buffer.
#[derive(Debug, Clone)]
pub struct BufferEntry {
    /// The event ID associated with this embedding.
    pub event_id: String,
    /// The embedding vector (pre-normalized by CandleEmbedder).
    pub embedding: Vec<f32>,
}

/// Fixed-capacity ring buffer for recent event embeddings.
///
/// Stores up to `capacity` embeddings and finds the most similar entry
/// via brute-force cosine similarity (dot product on normalized vectors).
/// When full, the oldest entry is overwritten.
#[derive(Debug)]
pub struct InFlightBuffer {
    entries: Vec<Option<BufferEntry>>,
    capacity: usize,
    dimension: usize,
    head: usize,
    count: usize,
}

impl InFlightBuffer {
    /// Create a new buffer with the given capacity and embedding dimension.
    pub fn new(capacity: usize, dimension: usize) -> Self {
        Self {
            entries: vec![None; capacity],
            capacity,
            dimension,
            head: 0,
            count: 0,
        }
    }

    /// Push a new entry into the buffer, overwriting the oldest if full.
    ///
    /// # Panics
    ///
    /// Panics if `embedding.len() != self.dimension`.
    pub fn push(&mut self, event_id: String, embedding: Vec<f32>) {
        assert_eq!(
            embedding.len(),
            self.dimension,
            "embedding dimension mismatch: expected {}, got {}",
            self.dimension,
            embedding.len()
        );
        self.entries[self.head] = Some(BufferEntry {
            event_id,
            embedding,
        });
        self.head = (self.head + 1) % self.capacity;
        if self.count < self.capacity {
            self.count += 1;
        }
    }

    /// Find the most similar entry above the given threshold.
    ///
    /// Returns `Some((event_id, similarity_score))` if a match is found,
    /// or `None` if no entry exceeds the threshold.
    pub fn find_similar(&self, query: &[f32], threshold: f32) -> Option<(String, f32)> {
        let mut best: Option<(String, f32)> = None;

        for entry in self.entries.iter().flatten() {
            let score = cosine_similarity(&entry.embedding, query);
            if score >= threshold
                && best
                    .as_ref()
                    .is_none_or(|(_, best_score)| score > *best_score)
            {
                best = Some((entry.event_id.clone(), score));
            }
        }

        best
    }

    /// Returns the number of entries currently in the buffer.
    pub fn len(&self) -> usize {
        self.count
    }

    /// Returns true if the buffer contains no entries.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Returns the maximum capacity of the buffer.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Clear all entries, resetting the buffer to empty.
    pub fn clear(&mut self) {
        for entry in &mut self.entries {
            *entry = None;
        }
        self.head = 0;
        self.count = 0;
    }
}

/// Compute cosine similarity between two vectors.
///
/// Since vectors are pre-normalized by CandleEmbedder, cosine similarity
/// is simply the dot product.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a normalized vector with a single non-zero component.
    fn unit_vector(dim: usize, index: usize) -> Vec<f32> {
        let mut v = vec![0.0; dim];
        v[index] = 1.0;
        v
    }

    #[test]
    fn test_push_and_find_exact_match() {
        let mut buf = InFlightBuffer::new(16, 4);
        let vec = vec![0.5, 0.5, 0.5, 0.5];
        buf.push("evt-1".to_string(), vec.clone());

        let result = buf.find_similar(&vec, 0.9);
        assert!(result.is_some());
        let (id, score) = result.unwrap();
        assert_eq!(id, "evt-1");
        assert!((score - 1.0).abs() < 0.01, "expected ~1.0, got {score}");
    }

    #[test]
    fn test_no_match_below_threshold() {
        let mut buf = InFlightBuffer::new(16, 4);
        // Push a vector along dimension 0
        buf.push("evt-1".to_string(), unit_vector(4, 0));

        // Query with orthogonal vector along dimension 1
        let query = unit_vector(4, 1);
        let result = buf.find_similar(&query, 0.5);
        assert!(result.is_none());
    }

    #[test]
    fn test_ring_buffer_overwrites_oldest() {
        let capacity = 3;
        let mut buf = InFlightBuffer::new(capacity, 4);

        // Push capacity + 1 entries; the first should be overwritten
        buf.push("evt-0".to_string(), unit_vector(4, 0));
        buf.push("evt-1".to_string(), unit_vector(4, 1));
        buf.push("evt-2".to_string(), unit_vector(4, 2));
        buf.push("evt-3".to_string(), unit_vector(4, 3)); // overwrites evt-0

        assert_eq!(buf.len(), capacity);

        // evt-0 (unit_vector dimension 0) should be gone
        let result = buf.find_similar(&unit_vector(4, 0), 0.9);
        assert!(
            result.is_none(),
            "oldest entry should have been overwritten"
        );

        // evt-3 (unit_vector dimension 3) should be findable
        let result = buf.find_similar(&unit_vector(4, 3), 0.9);
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, "evt-3");
    }

    #[test]
    fn test_empty_buffer_returns_none() {
        let buf = InFlightBuffer::new(16, 4);
        let result = buf.find_similar(&[0.5, 0.5, 0.5, 0.5], 0.5);
        assert!(result.is_none());
    }

    #[test]
    fn test_clear_resets_buffer() {
        let mut buf = InFlightBuffer::new(16, 4);
        buf.push("evt-1".to_string(), vec![0.5, 0.5, 0.5, 0.5]);
        assert!(!buf.is_empty());

        buf.clear();
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);

        let result = buf.find_similar(&[0.5, 0.5, 0.5, 0.5], 0.5);
        assert!(result.is_none());
    }

    #[test]
    #[should_panic(expected = "embedding dimension mismatch")]
    fn test_dimension_mismatch_panics() {
        let mut buf = InFlightBuffer::new(16, 4);
        buf.push("evt-1".to_string(), vec![0.5, 0.5, 0.5]); // wrong dimension
    }
}
