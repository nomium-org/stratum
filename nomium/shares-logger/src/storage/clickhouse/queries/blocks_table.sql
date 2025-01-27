CREATE TABLE IF NOT EXISTS blocks (
    account_name String,
    worker_id String,
    channel_id UInt32,
    block_hash String,
    ntime UInt32,
    received_at DateTime64(3, 'UTC') DEFAULT now64(3, 'UTC'),
    is_rewards_calculated Bool DEFAULT false
) ENGINE = MergeTree()
PARTITION BY toYYYYMMDD(received_at)
ORDER BY (worker_id, received_at, ntime)
SETTINGS index_granularity = 8192;