//! Column family definitions for RocksDB.
//!
//! Each column family isolates data with different access patterns:
//! - events: Append-only conversation events (Universal compaction)
//! - toc_nodes: TOC hierarchy nodes (default compaction)
//! - toc_latest: Latest TOC node version pointers (default compaction)
//! - grips: Excerpt-to-event links (default compaction)
//! - outbox: Queue for async index updates (FIFO compaction)
//! - checkpoints: Crash recovery checkpoints (default compaction)

use rocksdb::{ColumnFamilyDescriptor, Options};

/// Column family name for conversation events
pub const CF_EVENTS: &str = "events";

/// Column family name for TOC hierarchy nodes
pub const CF_TOC_NODES: &str = "toc_nodes";

/// Column family name for latest TOC node version pointers
pub const CF_TOC_LATEST: &str = "toc_latest";

/// Column family name for grips (excerpt + event pointers)
pub const CF_GRIPS: &str = "grips";

/// Column family name for outbox queue (async index updates)
pub const CF_OUTBOX: &str = "outbox";

/// Column family name for background job checkpoints
pub const CF_CHECKPOINTS: &str = "checkpoints";

/// Column family for topic records
pub const CF_TOPICS: &str = "topics";

/// Column family for topic-node links
pub const CF_TOPIC_LINKS: &str = "topic_links";

/// Column family for topic relationships
pub const CF_TOPIC_RELS: &str = "topic_rels";

/// All column family names
pub const ALL_CF_NAMES: &[&str] = &[
    CF_EVENTS,
    CF_TOC_NODES,
    CF_TOC_LATEST,
    CF_GRIPS,
    CF_OUTBOX,
    CF_CHECKPOINTS,
    CF_TOPICS,
    CF_TOPIC_LINKS,
    CF_TOPIC_RELS,
];

/// Create column family options for events (append-only, compressed)
fn events_options() -> Options {
    let mut opts = Options::default();
    // Zstd compression for space efficiency
    opts.set_compression_type(rocksdb::DBCompressionType::Zstd);
    opts
}

/// Create column family options for outbox (FIFO for queue behavior)
fn outbox_options() -> Options {
    let mut opts = Options::default();
    // FIFO compaction for queue-like workload per STOR-05
    opts.set_compaction_style(rocksdb::DBCompactionStyle::Fifo);
    // Set max table files size for FIFO (required)
    opts.set_fifo_compaction_options(&rocksdb::FifoCompactOptions::default());
    opts
}

/// Build all column family descriptors
pub fn build_cf_descriptors() -> Vec<ColumnFamilyDescriptor> {
    vec![
        ColumnFamilyDescriptor::new(CF_EVENTS, events_options()),
        ColumnFamilyDescriptor::new(CF_TOC_NODES, Options::default()),
        ColumnFamilyDescriptor::new(CF_TOC_LATEST, Options::default()),
        ColumnFamilyDescriptor::new(CF_GRIPS, Options::default()),
        ColumnFamilyDescriptor::new(CF_OUTBOX, outbox_options()),
        ColumnFamilyDescriptor::new(CF_CHECKPOINTS, Options::default()),
        ColumnFamilyDescriptor::new(CF_TOPICS, Options::default()),
        ColumnFamilyDescriptor::new(CF_TOPIC_LINKS, Options::default()),
        ColumnFamilyDescriptor::new(CF_TOPIC_RELS, Options::default()),
    ]
}
