CREATE TABLE IF NOT EXISTS shares (
    user_identity String,
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
    timestamp DateTime64(3) DEFAULT now64(3)
) ENGINE = MergeTree()
PARTITION BY toYYYYMMDD(timestamp)
ORDER BY (worker_id, timestamp, ntime)
SETTINGS index_granularity = 8192;