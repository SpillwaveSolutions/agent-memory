# Performance Tuning

Configure storage performance parameters for optimal agent-memory operation.

## Overview

Agent-memory uses RocksDB for storage. These settings control memory usage, write throughput, and background maintenance.

## Key Parameters

### Write Buffer Size (`write_buffer_size_mb`)

Controls how much data is buffered in memory before flushing to disk.

| Setting | Value | Memory Usage | Write Throughput | Use Case |
|---------|-------|--------------|------------------|----------|
| Low | 16 MB | ~20 MB | Lower | Constrained systems, Raspberry Pi |
| Balanced | 64 MB | ~80 MB | Good | Most users (default) |
| High | 128 MB | ~160 MB | Best | Heavy write workloads |
| Maximum | 256 MB | ~320 MB | Highest | Enterprise, SSD required |

**Memory calculation:** Actual memory = write_buffer_size_mb * 1.2 (overhead)

### Background Jobs (`max_background_jobs`)

Controls parallel threads for compaction and flushing.

| Setting | Jobs | CPU Impact | Compaction Speed | Use Case |
|---------|------|------------|------------------|----------|
| Minimal | 1 | Very low | Slowest | Single-core systems |
| Balanced | 4 | Moderate | Good | Most users (default) |
| Aggressive | 8 | Higher | Fast | Multi-core, heavy usage |
| Maximum | 16 | High | Fastest | High-end systems |

**Recommendation:** Set to number of CPU cores / 2

## Performance Profiles

### Balanced (Default)

Best for most users:

```toml
[storage]
write_buffer_size_mb = 64
max_background_jobs = 4
```

### Low Memory

For constrained systems (< 4GB RAM):

```toml
[storage]
write_buffer_size_mb = 16
max_background_jobs = 1
```

### High Performance

For heavy workloads on modern hardware:

```toml
[storage]
write_buffer_size_mb = 128
max_background_jobs = 8
```

## When to Tune

### Increase Write Buffer If:

- High write volume (> 1000 events/hour)
- SSD storage available
- Seeing "write stall" in logs
- Available RAM > 4GB

### Decrease Write Buffer If:

- Memory-constrained system
- Running on HDD
- Sharing resources with other apps
- Seeing OOM errors

### Increase Background Jobs If:

- Multi-core CPU (4+ cores)
- Compaction falling behind
- High read/write mix
- Storage is SSD

### Decrease Background Jobs If:

- Single/dual core CPU
- Sharing CPU with other apps
- Power-constrained (laptop)
- Storage is HDD

## SSD vs HDD Considerations

### SSD Configuration

SSDs benefit from higher parallelism:

```toml
[storage]
write_buffer_size_mb = 128
max_background_jobs = 8
# SSD-optimized compaction
target_file_size_base_mb = 64
level_compaction_dynamic_level_bytes = true
```

### HDD Configuration

HDDs prefer sequential access:

```toml
[storage]
write_buffer_size_mb = 64
max_background_jobs = 2  # Limit parallel I/O
# HDD-optimized settings
max_bytes_for_level_base_mb = 256
target_file_size_base_mb = 32
```

## Monitoring Performance

### Check Write Performance

```bash
# Monitor write latency
memory-daemon admin stats | grep write_latency

# Check flush rate
memory-daemon admin stats | grep flush_rate
```

### Check Compaction

```bash
# View compaction status
memory-daemon admin stats | grep compaction

# Check if compaction is falling behind
memory-daemon admin stats | grep pending_compaction
```

### Monitor Memory

```bash
# Check memory usage
memory-daemon admin stats | grep memory

# System memory
ps aux | grep memory-daemon | awk '{print $4}' # %MEM
```

### Monitor Disk I/O

```bash
# macOS
iostat -w 1 | grep disk0

# Linux
iostat -x 1 | grep sda
```

## Advanced RocksDB Tuning

For advanced users who need fine-grained control:

### Block Cache

```toml
[storage.advanced]
block_cache_size_mb = 256  # Read cache size
cache_index_and_filter_blocks = true
```

### Bloom Filters

```toml
[storage.advanced]
bloom_filter_bits_per_key = 10  # Faster point lookups
whole_key_filtering = true
```

### Compression

```toml
[storage.advanced]
compression = "lz4"  # Fast compression
bottommost_compression = "zstd"  # Better ratio for cold data
```

### Write Ahead Log

```toml
[storage.advanced]
wal_dir = "/fast-ssd/wal"  # Separate WAL to fast storage
wal_size_limit_mb = 1024
wal_ttl_seconds = 3600
```

## Troubleshooting

### "Write stall detected"

Compaction can't keep up with writes:

```toml
[storage]
max_background_jobs = 8  # Increase
write_buffer_size_mb = 128  # Increase
```

### High Memory Usage

Reduce buffers:

```toml
[storage]
write_buffer_size_mb = 32
max_write_buffer_number = 2
```

### Slow Reads

Increase cache:

```toml
[storage.advanced]
block_cache_size_mb = 512
bloom_filter_bits_per_key = 10
```

### High CPU Usage

Reduce background jobs:

```toml
[storage]
max_background_jobs = 2
```

## Configuration Examples

### Development Laptop

```toml
[storage]
path = "~/.memory-store"
write_buffer_size_mb = 32
max_background_jobs = 2
```

### Production Server

```toml
[storage]
path = "/data/memory-store"
write_buffer_size_mb = 128
max_background_jobs = 8

[storage.advanced]
block_cache_size_mb = 1024
bloom_filter_bits_per_key = 10
compression = "lz4"
```

### Raspberry Pi

```toml
[storage]
path = "~/.memory-store"
write_buffer_size_mb = 16
max_background_jobs = 1

[storage.advanced]
compression = "none"  # Save CPU
```
