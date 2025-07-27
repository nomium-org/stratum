use serde::Deserialize;
use config::{Config, ConfigError, Environment, File};
use std::sync::Once;
use once_cell::sync::Lazy;
use log::info;

static INIT: Once = Once::new();

#[derive(Debug, Deserialize, Clone)]
pub struct ClickhouseSettings {
    pub url: String,
    pub database: String,
    pub username: String,
    pub password: String,
    pub batch_size: usize,
    pub batch_flush_interval_secs: u64,
    pub pool_size: usize,
    pub base_retry_delay_ms: u64,
    pub max_retry_delay_secs: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProcessingSettings {
    pub primary_channel_buffer_size: usize,
    pub backup_check_interval_secs: u64,
    pub block_verification_max_retries: u8,
    pub block_verification_retry_delay_ms: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BitcoinRpcSettings {
    pub url: String,
    pub port: u16,
    pub user: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub clickhouse: ClickhouseSettings,
    pub processing: ProcessingSettings,
    pub bitcoin_rpc: BitcoinRpcSettings,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        
        let default_config = include_str!("default_config.toml");
        
        log::info!(target: "shares", "Logging environment variables with prefix SHARES_LOGGER:");
        log_environment_variables();
        log::info!(target: "shares", "Loading configuration from default_config.toml...");
        
        let builder = Config::builder()
            .add_source(File::from_str(
                default_config,
                config::FileFormat::Toml
            ))
            .add_source(
                Environment::with_prefix("SHARES_LOGGER")
                    .separator("__") //double "_"
            )
            .build()?;
        
        let settings = builder.try_deserialize::<Settings>();
        
        match &settings {
            Ok(s) => log::info!(target: "shares", "Loaded configuration: {:?}", s),
            Err(e) => log::error!("Failed to load configuration: {:?}", e),
        };
        
        settings
    }
}

pub static SETTINGS: Lazy<Settings> = Lazy::new(|| {
    INIT.call_once(|| {
        // Initialization code if needed
    });
    Settings::new().expect("Failed to load settings")
});

fn log_environment_variables() {
    for (key, value) in std::env::vars() {
        if key.starts_with("SHARES_LOGGER") {
            info!(target: "shares", "Environment variable: {} = {}", key, value);
        }
    }
}