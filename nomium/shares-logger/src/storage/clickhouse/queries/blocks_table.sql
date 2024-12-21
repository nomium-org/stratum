CREATE TABLE IF NOT EXISTS blocks (
    user_identity String,
    worker_id String,
    channel_id UInt32,
    block_hash String,
    ntime UInt32,
    found_at DateTime64(3) DEFAULT now64(3)
) ENGINE = MergeTree()
ORDER BY (user_identity, ntime)
SETTINGS index_granularity = 8192