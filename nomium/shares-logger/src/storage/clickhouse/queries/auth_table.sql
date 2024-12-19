CREATE TABLE IF NOT EXISTS worker_auth (
    account_name String,
    worker_number UInt32,
    is_success Bool,
    worker_id String,
    worker_name String,
    user_id String,
    account_id String,
    auth_time DateTime64(3) DEFAULT now64(3)
) ENGINE = ReplacingMergeTree(auth_time)
PRIMARY KEY (account_name, worker_number)
ORDER BY (account_name, worker_number)
SETTINGS index_granularity = 8192