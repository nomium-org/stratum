# Shares Logger

A Rust crate for logging mining shares data to ClickHouse database. This crate is used as an external dependency in `protocols/v2/roles-logic-sv2`.

## Overview

Shares Logger handles the collection, processing and storage of mining share submissions. It provides buffered logging with primary and backup channels to ensure reliable data capture even under high load.

## Structure

```
shares-logger/
├── src/
│   ├── config.rs         # Configuration settings for ClickHouse and logging
│   ├── lib.rs           # Core functionality and share logging implementation
│   ├── services/
│   │   ├── clickhouse.rs    # ClickHouse database interaction
│   │   ├── debug_log.rs     # Debug logging functionality
│   │   ├── difficulty.rs    # Mining difficulty calculations
│   │   └── share_processor.rs # Share data processing logic
└── Cargo.toml
```

## Key Features

- Asynchronous share processing
- Buffered logging with primary and backup channels
- ClickHouse database integration
- Mining difficulty calculations
- Debug logging capabilities

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
shares-logger = { path = "path/to/shares-logger" }
```

Basic example:
```rust
// Create and log a share
shares_logger::services::share_processor::ShareProcessor::prepare_share_log(/* params */);
shares_logger::log_share(share_log);
```

## Configuration

The crate uses a configuration structure that can be customized for:
- ClickHouse connection details
- Batch processing settings
- Buffer sizes
- Logging intervals

## Database Queries Examples

The crate creates a materialized view `mv_hash_rate_stats` that aggregates mining statistics. Here are some common query examples:

### Last Hour Statistics
Get hashrate and share count for the last 60 minutes per channel:

```bash
curl -X POST 'http://localhost:8123/' \
-H "X-ClickHouse-User: default" \
-H "X-ClickHouse-Key: 5555" \
-d "SELECT 
    channel_id,
    sum(share_count) as shares,
    sum(total_hashes) / greatest(1, dateDiff('second', min(min_timestamp), max(max_timestamp))) as hashrate
FROM mining.mv_hash_rate_stats 
WHERE period_start >= now() - INTERVAL 60 MINUTE
GROUP BY channel_id
FORMAT Pretty"
```

### 24-Hour Statistics
Get hashrate and share count for the last 24 hours per channel:

```bash
curl -X POST 'http://localhost:8123/' \
-H "X-ClickHouse-User: default" \
-H "X-ClickHouse-Key: 5555" \
-d "SELECT 
    channel_id,
    sum(share_count) as shares,
    sum(total_hashes) / greatest(1, dateDiff('second', min(min_timestamp), max(max_timestamp))) as hashrate
FROM mining.mv_hash_rate_stats 
WHERE period_start >= now() - INTERVAL 24 HOUR
GROUP BY channel_id
FORMAT Pretty"
```

Query results show:
- `channel_id`: Unique identifier for each mining channel
- `shares`: Total number of shares submitted
- `hashrate`: Average hashrate in hashes per second

The materialized view automatically aggregates data per minute, making these queries efficient for real-time monitoring and historical analysis.
