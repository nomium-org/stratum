CREATE MATERIALIZED VIEW IF NOT EXISTS mv_hash_rate_stats
ENGINE = SummingMergeTree()
PARTITION BY toYYYYMMDD(period_start)
ORDER BY (worker_id, period_start)
POPULATE
AS
SELECT
    worker_id,
    toStartOfMinute(timestamp) as period_start,
    sum(difficulty * pow(2, 32)) as total_hashes
FROM shares
GROUP BY worker_id, period_start;
