use once_cell::sync::Lazy;

pub struct NomiumSharesConfig {
    
    /// URL of the ClickHouse server
    /// Example: "http://localhost:8123"
    pub clickhouse_url: String,

    /// Name of the database in ClickHouse
    /// Example: "mining"
    pub clickhouse_database: String,

    /// Username for ClickHouse authentication
    /// Example: "default"
    pub clickhouse_username: String,

    /// Password for ClickHouse authentication
    pub clickhouse_password: String,

    /// Number of shares to accumulate before writing to ClickHouse
    /// Larger values improve performance but increase memory usage and latency
    /// Range: 1-10000
    pub clickhouse_batch_size: usize,

    /// Size of the primary channel buffer for share processing
    /// Controls how many shares can be queued before falling back to backup channel
    /// Range: 10-1000, Default: 100
    pub primary_channel_buffer_size: usize,

    /// Interval in seconds for checking and processing shares in backup channel
    /// Lower values reduce latency but increase CPU usage
    /// Range: 1-60, Default: 1
    pub backup_check_interval_secs: u64,

    /// Maximum time in seconds to hold shares in batch before forced flush
    /// Lower values reduce latency but may impact performance
    /// Range: 1-300, Default: 5
    pub batch_flush_interval_secs: u64,
}

pub static CONFIG: Lazy<NomiumSharesConfig> = Lazy::new(|| {
    NomiumSharesConfig {
        clickhouse_url: "http://localhost:8123".to_string(),
        clickhouse_database: "mining".to_string(),
        clickhouse_username: "default".to_string(),
        clickhouse_password: "5555".to_string(),
        clickhouse_batch_size: 2,
        primary_channel_buffer_size: 100,
        backup_check_interval_secs: 1,
        batch_flush_interval_secs: 5,
    }
});