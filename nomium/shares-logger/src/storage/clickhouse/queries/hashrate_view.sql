CREATE MATERIALIZED VIEW IF NOT EXISTS mv_hash_rate_stats
ENGINE = SummingMergeTree()
PARTITION BY toYYYYMMDD(period_start)
ORDER BY (channel_id, period_start)
AS
SELECT
    channel_id,
    toStartOfMinute(timestamp) as period_start,
    count() as share_count,
    sum(difficulty * pow(2, 32)) as total_hashes,
    min(timestamp) as min_timestamp,
    max(timestamp) as max_timestamp
FROM shares
GROUP BY channel_id, period_start