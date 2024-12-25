CREATE MATERIALIZED VIEW IF NOT EXISTS mv_hash_rate_stats
ENGINE = SummingMergeTree()
PARTITION BY toYYYYMMDD(period_start)
ORDER BY (worker_id, period_start)
POPULATE
AS
SELECT
    worker_id,
    toStartOfMinute(timestamp) AS period_start,
    count() AS share_count,
    min(difficulty) * count() * pow(2, 32) AS total_hashes,
    sum(CASE WHEN share_status = 0 THEN 1 ELSE 0 END) AS refused_shards,
    max(timestamp) AS max_timestamp
FROM shares
GROUP BY worker_id, period_start;
