CREATE TABLE IF NOT EXISTS blocks (
    channel_id UInt32,
    block_hash String,
    ntime UInt32,
    found_at DateTime64(3) DEFAULT now64(3)
) ENGINE = MergeTree()
ORDER BY (channel_id, ntime)
SETTINGS index_granularity = 8192