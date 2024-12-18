CREATE MATERIALIZED VIEW IF NOT EXISTS mv_hash_rate_stats
ENGINE = SummingMergeTree()
PARTITION BY toYYYYMMDD(period_start)
ORDER BY (user_identity, period_start)
AS
SELECT
    user_identity,
    toStartOfMinute(timestamp) as period_start,
    count() as share_count,
    sum(difficulty * pow(2, 32)) as total_hashes,
    min(timestamp) as min_timestamp,
    max(timestamp) as max_timestamp,
    avgWeighted(difficulty, difficulty) as avg_difficulty
FROM shares
GROUP BY user_identity, period_start;
