use serde::Deserialize;
use config::{Config, ConfigError, Environment, File};
use std::sync::Once;
use once_cell::sync::Lazy;

static INIT: Once = Once::new();

#[derive(Debug, Deserialize, Clone)]
pub struct ClickhouseSettings {
    pub url: String,
    pub database: String,
    pub username: String,
    pub password: String,
    pub batch_size: usize,
    pub batch_flush_interval_secs: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProcessingSettings {
    pub primary_channel_buffer_size: usize,
    pub backup_check_interval_secs: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub clickhouse: ClickhouseSettings,
    pub processing: ProcessingSettings,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let default_config = include_str!("default_config.toml");
        
        let mut builder = Config::builder();
        
        builder = builder.add_source(File::from_str(
            default_config,
            config::FileFormat::Toml
        ));

        // examples: 
        // export SHARES_LOGGER_CLICKHOUSE_URL="http://custom-host:8123"
        // export SHARES_LOGGER_CLICKHOUSE_BATCH_FLUSH_INTERVAL_SECS="10"
        builder = builder.add_source(
            Environment::with_prefix("SHARES_LOGGER")
                .separator("_")
        );

        builder.build()?.try_deserialize()
    }
}

pub static SETTINGS: Lazy<Settings> = Lazy::new(|| {
    INIT.call_once(|| {
        // 
    });

    Settings::new().expect("Failed to load settings")
});