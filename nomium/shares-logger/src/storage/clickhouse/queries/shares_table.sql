CREATE TABLE IF NOT EXISTS shares (
    channel_id UInt32,
    sequence_number UInt32,
    job_id UInt32,
    nonce UInt32,
    ntime UInt32,
    version UInt32,
    hash String,
    share_status UInt8,
    extranonce String,
    difficulty Float64,
    timestamp DateTime64(3) DEFAULT now64(3)
) ENGINE = MergeTree()
PARTITION BY toYYYYMMDD(timestamp)
PRIMARY KEY (channel_id, timestamp)
ORDER BY (channel_id, timestamp, sequence_number)
SETTINGS index_granularity = 8192