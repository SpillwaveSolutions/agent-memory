# Phase 2: TOC Building - Research

**Researched:** 2026-01-30
**Domain:** Event segmentation, LLM summarization, time hierarchy construction, background job processing
**Confidence:** HIGH

## Summary

Phase 2 builds the Table of Contents (TOC) - the hierarchical navigation structure enabling agents to find conversations without brute-force scanning. Research focused on four areas: event segmentation (time/token boundaries with overlap), LLM summarization (pluggable trait supporting API and local inference), time hierarchy construction (Year→Month→Week→Day→Segment), and background job checkpointing.

The standard approach uses time-gap detection (30 min) combined with token counting (4K) for segment boundaries, with overlap windows (5 min or 500 tokens) for context continuity. Summarization is async via a pluggable trait supporting OpenAI/Claude APIs or local models. TOC nodes are built bottom-up from segments, with rollup jobs aggregating children at each level. Checkpoints ensure crash recovery.

**Primary recommendation:** Start with segmentation engine (02-01), then summarizer trait with API implementation (02-02), finally TOC hierarchy builder with rollup jobs (02-03).

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tiktoken-rs | 0.5+ | Token counting for OpenAI models | Accurate token estimation, used by openai-api-rs |
| async-trait | 0.1 | Async traits for Summarizer | Standard async abstraction for traits |
| reqwest | 0.12 | HTTP client for API calls | De facto Rust HTTP client |
| serde_json | 1.0 | JSON serialization | API request/response handling |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tokio | 1.49 | Async runtime | Background job execution |
| backoff | 0.4 | Retry with exponential backoff | API rate limiting |
| secrecy | 0.10 | Secret string handling | API key storage |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| tiktoken-rs | tokenizers | tokenizers is HuggingFace-focused, tiktoken matches OpenAI exactly |
| reqwest | hyper | hyper is lower-level, reqwest has better ergonomics |
| Manual retry | tower::retry | tower adds complexity for simple API calls |

## Architecture Patterns

### Pattern 1: Time-Gap Segmentation

**What:** Detect segment boundaries based on time gaps between events.
**When to use:** Primary segmentation trigger (TOC-03).
**Example:**
```rust
/// Segment boundary detection based on time gaps
pub struct SegmentationConfig {
    /// Maximum time gap before starting new segment (TOC-03: 30 min)
    pub time_threshold_ms: i64,
    /// Maximum tokens before starting new segment (TOC-03: 4K)
    pub token_threshold: usize,
    /// Overlap for context continuity (TOC-04: 5 min)
    pub overlap_time_ms: i64,
    /// Overlap tokens (TOC-04: 500 tokens)
    pub overlap_tokens: usize,
}

impl Default for SegmentationConfig {
    fn default() -> Self {
        Self {
            time_threshold_ms: 30 * 60 * 1000, // 30 minutes
            token_threshold: 4000,
            overlap_time_ms: 5 * 60 * 1000, // 5 minutes
            overlap_tokens: 500,
        }
    }
}
```

### Pattern 2: Segment with Overlap

**What:** Include overlap events from previous segment for context continuity.
**When to use:** All segmentation (TOC-04).
**Example:**
```rust
/// A segment of events with optional overlap from previous segment
pub struct Segment {
    /// Unique segment identifier
    pub segment_id: String,
    /// Events in the overlap window (from previous segment)
    pub overlap_events: Vec<Event>,
    /// Events in this segment (excluding overlap)
    pub events: Vec<Event>,
    /// Start time of the segment (excluding overlap)
    pub start_time: DateTime<Utc>,
    /// End time of the segment
    pub end_time: DateTime<Utc>,
    /// Token count (excluding overlap)
    pub token_count: usize,
}
```

### Pattern 3: Pluggable Summarizer Trait

**What:** Async trait for generating summaries from events.
**When to use:** All summarization (SUMM-01, SUMM-02).
**Example:**
```rust
/// Output from summarization
pub struct Summary {
    pub title: String,
    pub bullets: Vec<String>,
    pub keywords: Vec<String>,
}

/// Pluggable summarizer trait (SUMM-01)
#[async_trait::async_trait]
pub trait Summarizer: Send + Sync {
    /// Generate a summary from events (SUMM-02)
    async fn summarize_events(&self, events: &[Event]) -> Result<Summary, SummarizerError>;

    /// Generate a rollup summary from child summaries (SUMM-04)
    async fn summarize_children(&self, summaries: &[Summary]) -> Result<Summary, SummarizerError>;
}
```

### Pattern 4: API-Based Summarizer

**What:** Summarizer implementation using OpenAI/Claude API.
**When to use:** Default production summarizer.
**Example:**
```rust
pub struct ApiSummarizer {
    client: reqwest::Client,
    api_key: secrecy::SecretString,
    model: String,
    base_url: String,
}

#[async_trait::async_trait]
impl Summarizer for ApiSummarizer {
    async fn summarize_events(&self, events: &[Event]) -> Result<Summary, SummarizerError> {
        let prompt = build_events_prompt(events);
        let response = self.call_api(&prompt).await?;
        parse_summary_response(&response)
    }

    async fn summarize_children(&self, summaries: &[Summary]) -> Result<Summary, SummarizerError> {
        let prompt = build_rollup_prompt(summaries);
        let response = self.call_api(&prompt).await?;
        parse_summary_response(&response)
    }
}
```

### Pattern 5: TOC Node ID Format

**What:** Hierarchical node IDs that encode level and time period.
**When to use:** All TOC node creation.
**Example:**
```rust
/// Generate node ID based on level and time period
pub fn generate_node_id(level: TocLevel, time: DateTime<Utc>) -> String {
    match level {
        TocLevel::Year => format!("toc:year:{}", time.year()),
        TocLevel::Month => format!("toc:month:{}:{:02}", time.year(), time.month()),
        TocLevel::Week => {
            let week = time.iso_week();
            format!("toc:week:{}:W{:02}", week.year(), week.week())
        }
        TocLevel::Day => format!("toc:day:{}", time.format("%Y-%m-%d")),
        TocLevel::Segment => format!("toc:segment:{}:{}",
            time.format("%Y-%m-%d"),
            ulid::Ulid::new()
        ),
    }
}
```

### Pattern 6: Rollup Job with Checkpointing

**What:** Background job that aggregates child nodes into parent summaries.
**When to use:** Day/Week/Month rollup (TOC-05).
**Example:**
```rust
/// Checkpoint for crash recovery (STOR-03, TOC-05)
#[derive(Serialize, Deserialize)]
pub struct RollupCheckpoint {
    pub job_name: String,
    pub level: TocLevel,
    pub last_processed_time: DateTime<Utc>,
    pub processed_count: usize,
}

pub async fn run_rollup_job(
    storage: &Storage,
    summarizer: &dyn Summarizer,
    level: TocLevel,
) -> Result<(), Error> {
    let job_name = format!("rollup_{}", level);

    // Load checkpoint for crash recovery
    let checkpoint = load_checkpoint(storage, &job_name)?;
    let start_time = checkpoint.map(|c| c.last_processed_time).unwrap_or(DateTime::UNIX_EPOCH);

    // Find nodes at child level that need rollup
    let child_level = level.child().ok_or(Error::NoChildLevel)?;
    let children = get_nodes_since(storage, child_level, start_time)?;

    // Group by parent period and summarize
    for (parent_id, child_nodes) in group_by_parent(children) {
        let summaries: Vec<Summary> = child_nodes.iter()
            .map(|n| Summary { title: n.title.clone(), bullets: n.bullets.clone(), keywords: n.keywords.clone() })
            .collect();

        let rollup = summarizer.summarize_children(&summaries).await?;
        let parent_node = create_or_update_node(storage, &parent_id, level, rollup)?;

        // Save checkpoint after each parent
        save_checkpoint(storage, &job_name, parent_node.end_time)?;
    }

    Ok(())
}
```

### Pattern 7: Versioned TOC Nodes

**What:** Append new version instead of mutating existing node.
**When to use:** All TOC updates (TOC-06).
**Example:**
```rust
/// Storage keys for versioned TOC nodes
/// toc_nodes CF: "toc:{node_id}:v{version}" -> TocNode bytes
/// toc_latest CF: "latest:{node_id}" -> latest version number

pub fn put_toc_node(storage: &Storage, node: &TocNode) -> Result<(), Error> {
    let nodes_cf = storage.cf_handle(CF_TOC_NODES)?;
    let latest_cf = storage.cf_handle(CF_TOC_LATEST)?;

    // Get current version
    let latest_key = format!("latest:{}", node.node_id);
    let current_version = storage.get(&latest_cf, &latest_key)?
        .map(|b| u32::from_be_bytes(b.try_into().unwrap()))
        .unwrap_or(0);

    let new_version = current_version + 1;
    let versioned_key = format!("toc:{}:v{}", node.node_id, new_version);

    let mut node = node.clone();
    node.version = new_version;

    let mut batch = WriteBatch::default();
    batch.put_cf(&nodes_cf, versioned_key, node.to_bytes()?);
    batch.put_cf(&latest_cf, latest_key, new_version.to_be_bytes());
    storage.write(batch)?;

    Ok(())
}
```

## Common Pitfalls

### Pitfall 1: Token Count Explosion with Tool Results

**What goes wrong:** Tool results (file contents, command output) inflate token counts dramatically.
**Why it happens:** Naive counting includes full tool output text.
**How to avoid:** Truncate or summarize tool results before counting.
```rust
fn count_tokens_for_event(event: &Event) -> usize {
    let text = if event.event_type == EventType::ToolResult {
        // Truncate tool results to reasonable length
        &event.text[..event.text.len().min(1000)]
    } else {
        &event.text
    };
    tiktoken_rs::cl100k_base().unwrap().encode_with_special_tokens(text).len()
}
```

### Pitfall 2: Overlapping Segments Miss Context

**What goes wrong:** Overlap window too small, summarizer lacks context.
**Why it happens:** Events referenced in current segment occurred in overlap period.
**How to avoid:** Include overlap events in summarization input, mark them as context.
```rust
fn prepare_for_summarization(segment: &Segment) -> Vec<Event> {
    let mut all_events = segment.overlap_events.clone();
    all_events.extend(segment.events.clone());
    // Mark overlap events for summarizer
    for event in &mut all_events[..segment.overlap_events.len()] {
        event.metadata.insert("_overlap".to_string(), "true".to_string());
    }
    all_events
}
```

### Pitfall 3: API Rate Limiting Crashes Job

**What goes wrong:** Burst of summarization calls hits rate limit, job fails.
**Why it happens:** No backoff or throttling.
**How to avoid:** Use exponential backoff with jitter.
```rust
use backoff::{ExponentialBackoff, retry};

async fn call_api_with_retry<T, F, Fut>(f: F) -> Result<T, Error>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, Error>>,
{
    let backoff = ExponentialBackoff::default();
    retry(backoff, || async { f().await.map_err(backoff::Error::transient) }).await
}
```

### Pitfall 4: Checkpoint Not Saved Atomically

**What goes wrong:** Crash between processing and checkpoint, work repeated or lost.
**Why it happens:** Checkpoint written separately from node update.
**How to avoid:** Write node and checkpoint in same atomic batch.

### Pitfall 5: Rollup Runs Before Segments Complete

**What goes wrong:** Day rollup runs before all segments for that day exist.
**Why it happens:** New events still arriving, rollup job triggered too early.
**How to avoid:** Only rollup periods that are "closed" (current period excluded).
```rust
fn should_rollup_period(period_end: DateTime<Utc>) -> bool {
    // Only rollup if period ended at least 1 hour ago
    period_end + Duration::hours(1) < Utc::now()
}
```

## Code Examples

### Segment Builder

```rust
pub struct SegmentBuilder {
    config: SegmentationConfig,
    current_events: Vec<Event>,
    current_tokens: usize,
    last_event_time: Option<DateTime<Utc>>,
}

impl SegmentBuilder {
    pub fn new(config: SegmentationConfig) -> Self {
        Self {
            config,
            current_events: Vec::new(),
            current_tokens: 0,
            last_event_time: None,
        }
    }

    /// Add an event, returns Some(Segment) if boundary reached
    pub fn add_event(&mut self, event: Event) -> Option<Segment> {
        let event_tokens = count_tokens(&event.text);

        // Check time gap boundary
        if let Some(last_time) = self.last_event_time {
            let gap_ms = event.timestamp.timestamp_millis() - last_time.timestamp_millis();
            if gap_ms > self.config.time_threshold_ms {
                return Some(self.flush_segment());
            }
        }

        // Check token boundary
        if self.current_tokens + event_tokens > self.config.token_threshold {
            return Some(self.flush_segment());
        }

        self.current_events.push(event.clone());
        self.current_tokens += event_tokens;
        self.last_event_time = Some(event.timestamp);

        None
    }

    /// Flush current events as a segment
    fn flush_segment(&mut self) -> Segment {
        let events = std::mem::take(&mut self.current_events);
        let start_time = events.first().map(|e| e.timestamp).unwrap_or_else(Utc::now);
        let end_time = events.last().map(|e| e.timestamp).unwrap_or_else(Utc::now);

        self.current_tokens = 0;
        self.last_event_time = None;

        Segment {
            segment_id: format!("seg:{}", ulid::Ulid::new()),
            overlap_events: Vec::new(), // Filled by caller
            events,
            start_time,
            end_time,
            token_count: self.current_tokens,
        }
    }
}
```

### OpenAI API Summarizer Prompt

```rust
fn build_events_prompt(events: &[Event]) -> String {
    let events_text: String = events.iter()
        .map(|e| format!("[{}] {}: {}", e.timestamp, e.role, e.text))
        .collect::<Vec<_>>()
        .join("\n\n");

    format!(r#"Summarize this conversation segment for a Table of Contents entry.

CONVERSATION:
{events_text}

Provide your response in JSON format:
{{
  "title": "Brief title (5-10 words)",
  "bullets": ["Key point 1", "Key point 2", "Key point 3"],
  "keywords": ["keyword1", "keyword2", "keyword3"]
}}

Guidelines:
- Title should capture the main topic or activity
- 3-5 bullet points summarizing key discussions or decisions
- 3-7 keywords for search/filtering
- Focus on what would help someone find this conversation later"#)
}
```

## Open Questions

1. **Summarization API Choice**
   - What we know: OpenAI and Claude APIs both work well
   - What's unclear: Which model is best for summarization (gpt-4o-mini vs claude-3-haiku)
   - Recommendation: Start with gpt-4o-mini (cheaper, sufficient quality), make model configurable

2. **Real-time vs Batch Segmentation**
   - What we know: Events arrive continuously via hooks
   - What's unclear: Process immediately or batch?
   - Recommendation: Batch via outbox processing - check outbox periodically, process segments in batches

3. **Rollup Frequency**
   - What we know: Need rollup for Day→Week→Month→Year
   - What's unclear: How often to run rollup jobs
   - Recommendation: Day rollup hourly, Week/Month/Year daily via cron-like scheduler

## Sources

### Primary (HIGH confidence)

- tiktoken-rs documentation - Token counting
- OpenAI API documentation - Chat completions
- Phase 1 research - Storage patterns, checkpointing

### Secondary (MEDIUM confidence)

- Claude API documentation - Alternative summarizer
- backoff crate documentation - Retry patterns

---
*Generated by GSD Phase Researcher, 2026-01-30*
