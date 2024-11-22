# Shares Logger

A Rust crate for logging mining shares data to ClickHouse database. This crate is used as an external dependency in `protocols/v2/roles-logic-sv2`.

## Overview

Shares Logger handles the collection, processing and storage of mining share submissions. It provides buffered logging with primary and backup channels to ensure reliable data capture even under high load.

## Structure

```
shares-logger/
├── src/
│   ├── config/
│   │   ├── mod.rs           # Configuration module exports
│   │   ├── settings.rs      # Configuration implementation 
│   │   └── default_config.toml  # Default configuration values
│   ├── errors/
│   │   ├── mod.rs           # Error types module exports
│   │   └── clickhouse.rs    # ClickHouse-specific error types
│   ├── models/
│   │   ├── mod.rs           # Models module exports
│   │   ├── share_log.rs     # Share logging data structure
│   │   └── clickhouse_share.rs  # ClickHouse-specific share model
│   ├── services/
│   │   ├── mod.rs           # Services module exports
│   │   ├── difficulty.rs    # Mining difficulty calculations
│   │   └── share_processor.rs    # Share data processing logic
│   ├── storage/
│   │   ├── mod.rs           # Storage module exports
│   │   └── clickhouse/      # ClickHouse storage implementation
│   │       ├── mod.rs       # ClickHouse module exports
│   │       └── service.rs   # ClickHouse storage service implementation with batching and flush logic
│   ├── traits/
│   │   ├── mod.rs           # Traits module exports
│   │   └── storage.rs       # Storage trait definitions
│   └── lib.rs               # Library root and share logging implementation
├── tests/
│   ├── common/
│   │   ├── mod.rs           # Test utilities module exports
│   │   └── mock_storage.rs  # Mock storage for testing
│   └── highload_test.rs     # High-load testing scenarios
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
The crate supports configuration through both TOML files and environment variables.

### Environment Variables
All configuration options can be overridden using environment variables with the prefix `SHARES_LOGGER__`. 
Use double underscores (`__`) as separators for nested configuration values.

Examples:
- `SHARES_LOGGER__CLICKHOUSE__URL=http://localhost:8123`
- `SHARES_LOGGER__CLICKHOUSE__DATABASE=mining`
- `SHARES_LOGGER__CLICKHOUSE__USERNAME=default`
- `SHARES_LOGGER__CLICKHOUSE__PASSWORD=5555`
- `SHARES_LOGGER__CLICKHOUSE__BATCH_SIZE=2`
- `SHARES_LOGGER__CLICKHOUSE__BATCH_FLUSH_INTERVAL_SECS=5`
- `SHARES_LOGGER__PROCESSING__PRIMARY_CHANNEL_BUFFER_SIZE=100`
- `SHARES_LOGGER__PROCESSING__BACKUP_CHECK_INTERVAL_SECS=1`

### Default Configuration
Default values are specified in `config/default_config.toml`:

```toml
[clickhouse]
url = "http://localhost:8123"
database = "mining"
username = "default"
password = "5555"
batch_size = 2
batch_flush_interval_secs = 5

[processing]
primary_channel_buffer_size = 100
backup_check_interval_secs = 1

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

## Known Limitations and Future Improvements

This code is a prototype implementation and has several known issues that should be addressed for production use:

### Error Handling
1. Asynchronous Channel Communication
   - Limited error handling for channel transmission failures
   - No proper recovery mechanism for channel overflow situations
   - Missing backpressure handling

2. Database Operations
   - Basic error handling for database operations
   - No retry mechanism for failed database writes
   - Missing transaction support for batch operations
   - No connection pool implementation
   - Limited connection error recovery

3. Database Schema
   - Current schema needs optimization for large-scale deployments
   - Index strategy might need adjustment based on query patterns
   - Materialized view refresh strategy needs review
   - Partition strategy might need optimization for long-term data storage

These limitations are known and documented for future improvements.

## Testing

### High-Load Testing
The crate includes comprehensive high-load testing to ensure reliable performance under stress conditions.

#### Purpose
The high-load test (`tests/highload_test.rs`) verifies:
- Correct handling of rapid share submissions
- Proper functioning of the primary and backup channels
- Data integrity during high-frequency operations
- Sequential processing of shares
- No data loss under load

#### Running Tests
Execute the high-load test using:
```bash
cargo test test_highload_share_processing -- --nocapture
```