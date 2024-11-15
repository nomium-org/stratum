use once_cell::sync::Lazy;

pub struct NomiumSharesConfig {
    pub clickhouse_url: String,
    pub clickhouse_database: String,
    pub clickhouse_username: String,
    pub clickhouse_password: String,
    pub clickhouse_batch_size: usize,
    pub primary_channel_buffer_size: usize,
    pub backup_check_interval_secs: u64,
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