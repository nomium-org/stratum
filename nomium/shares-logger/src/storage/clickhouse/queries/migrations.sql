-- 27 января

DROP TABLE shares;
DROP TABLE mv_hash_rate_stats;
DROP TABLE blocks;

CREATE TABLE IF NOT EXISTS shares (
    account_name String,
    worker_id String,
    sequence_number UInt32,
    job_id UInt32,
    nonce UInt32,
    time_from_worker UInt32,
    version UInt32,
    hash String,
    share_status UInt8,
    extranonce String,
    difficulty Float64,
    channel_id UInt32,
    is_pps_reward_calculated Bool DEFAULT false,
    received_at DateTime64(3, 'UTC') DEFAULT now64(3, 'UTC')
) ENGINE = MergeTree()
PARTITION BY toYYYYMMDD(received_at)
ORDER BY (worker_id, received_at, time_from_worker)
SETTINGS index_granularity = 8192;

CREATE MATERIALIZED VIEW IF NOT EXISTS mv_hash_rate_stats
ENGINE = SummingMergeTree()
PARTITION BY toYYYYMMDD(period_start)
ORDER BY (worker_id, period_start)
POPULATE
AS
SELECT
    worker_id,
    toStartOfMinute(received_at, 'UTC') AS period_start,
    count() AS share_count,
    min(difficulty) * count() * pow(2, 32) AS total_hashes,
    sum(CASE WHEN share_status = 0 THEN 1 ELSE 0 END) AS refused_shards,
    max(received_at) AS max_received_at
FROM shares
GROUP BY worker_id, period_start;

CREATE TABLE IF NOT EXISTS blocks (
    account_name String,
    worker_id String,
    channel_id UInt32,
    block_hash String,
    time_from_worker UInt32,
    received_at DateTime64(3, 'UTC') DEFAULT now64(3, 'UTC'),
    is_rewards_calculated Bool DEFAULT false
) ENGINE = MergeTree()
PARTITION BY toYYYYMMDD(received_at)
ORDER BY (worker_id, received_at, time_from_worker)
SETTINGS index_granularity = 8192;

-- 16 января 2025 миграция

ALTER TABLE shares
ADD COLUMN IF NOT EXISTS is_pps_reward_calculated Bool DEFAULT false;

-- 14 января 2025 миграция

-- shares

ALTER TABLE shares 
MODIFY COLUMN timestamp DateTime64(3, 'UTC');

CREATE TABLE IF NOT EXISTS shares_new (
    account_name String,
    worker_id String,
    sequence_number UInt32,
    job_id UInt32,
    nonce UInt32,
    ntime UInt32,
    version UInt32,
    hash String,
    share_status UInt8,
    extranonce String,
    difficulty Float64,
    channel_id UInt32,
    timestamp DateTime64(3, 'UTC') DEFAULT now64(3, 'UTC')
) ENGINE = MergeTree()
PARTITION BY toYYYYMMDD(timestamp)
ORDER BY (worker_id, timestamp, ntime)
SETTINGS index_granularity = 8192;

INSERT INTO shares_new 
SELECT
    account_name,
    worker_id,
    sequence_number,
    job_id,
    nonce,
    ntime,
    version,
    hash,
    share_status,
    extranonce,
    difficulty,
    channel_id,
    timestamp - INTERVAL 3 HOUR as timestamp
FROM shares;

RENAME TABLE 
    shares TO shares_old,
    shares_new TO shares;

DROP TABLE shares_old;


-- blocks

ALTER TABLE blocks 
MODIFY COLUMN found_at DateTime64(3, 'UTC');

CREATE TABLE IF NOT EXISTS blocks_new (
    account_name String,
    worker_id String,
    channel_id UInt32,
    block_hash String,
    ntime UInt32,
    found_at DateTime64(3, 'UTC') DEFAULT now64(3, 'UTC'),
    is_rewards_calculated Bool DEFAULT false
) ENGINE = MergeTree()
PARTITION BY toYYYYMMDD(found_at)
ORDER BY (worker_id, found_at, ntime)
SETTINGS index_granularity = 8192;

INSERT INTO blocks_new 
SELECT
    account_name,
    worker_id,
    channel_id,
    block_hash,
    ntime,
    found_at - INTERVAL 3 HOUR as found_at,
    is_rewards_calculated
FROM blocks;

RENAME TABLE 
    blocks TO blocks_old,
    blocks_new TO blocks;

DROP TABLE blocks_old;


-- Пересоздаем mv_hash_rate_stats

DROP TABLE IF EXISTS mv_hash_rate_stats;

CREATE MATERIALIZED VIEW IF NOT EXISTS mv_hash_rate_stats
ENGINE = SummingMergeTree()
PARTITION BY toYYYYMMDD(period_start)
ORDER BY (worker_id, period_start)
POPULATE
AS
SELECT
    worker_id,
    toStartOfMinute(timestamp, 'UTC') AS period_start,
    count() AS share_count,
    min(difficulty) * count() * pow(2, 32) AS total_hashes,
    sum(CASE WHEN share_status = 0 THEN 1 ELSE 0 END) AS refused_shards,
    max(timestamp) AS max_timestamp
FROM shares
GROUP BY worker_id, period_start;

-- 7 января 2025
--
ALTER TABLE blocks
ADD COLUMN is_rewards_calculated Bool DEFAULT false;

--
--
ALTER TABLE shares ADD COLUMN account_name String;
ALTER TABLE blocks ADD COLUMN account_name String;

--
-- 
ALTER TABLE shares DROP COLUMN user_identity;
ALTER TABLE blocks DROP COLUMN user_identity;