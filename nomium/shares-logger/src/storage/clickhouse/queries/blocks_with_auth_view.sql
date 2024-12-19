CREATE MATERIALIZED VIEW IF NOT EXISTS mv_blocks_with_auth
ENGINE = ReplacingMergeTree()
ORDER BY (user_identity, found_at)
AS
SELECT 
    b.*,
    wa.worker_id,
    wa.worker_name,
    wa.user_id,
    wa.account_id
FROM mining.blocks b
LEFT JOIN (
    SELECT 
        concat(account_name, '.', toString(worker_number)) as user_identity,
        worker_id,
        worker_name,
        user_id,
        account_id,
        auth_time
    FROM (
        SELECT 
            account_name,
            worker_number,
            worker_id,
            worker_name,
            user_id,
            account_id,
            auth_time,
            row_number() OVER (PARTITION BY account_name, worker_number ORDER BY auth_time DESC) as rn
        FROM mining.worker_auth
    ) ranked
    WHERE rn = 1
) wa USING (user_identity)