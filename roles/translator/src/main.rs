#![allow(special_module_name)]
mod args;
mod lib;

use args::Args;
use error::{Error, ProxyResult};
pub use lib::{downstream_sv1, error, proxy, proxy_config, status, upstream_sv2};
use proxy_config::ProxyConfig;
use dotenvy::dotenv;
use tracing_subscriber::prelude::*;
use tracing::Level;
use std::str::FromStr;
use crate::lib::metrics;

use ext_config::{Config, File, FileFormat};

use tracing::{error, info};

/// Process CLI args, if any.
#[allow(clippy::result_large_err)]
fn process_cli_args<'a>() -> ProxyResult<'a, ProxyConfig> {
    // Parse CLI arguments
    let args = Args::from_args().map_err(|help| {
        error!("{}", help);
        Error::BadCliArgs
    })?;

    // Build configuration from the provided file path
    let config_path = args.config_path.to_str().ok_or_else(|| {
        error!("Invalid configuration path.");
        Error::BadCliArgs
    })?;

    let settings = Config::builder()
        .add_source(File::new(config_path, FileFormat::Toml))
        .build()?;

    // Deserialize settings into ProxyConfig
    let config = settings.try_deserialize::<ProxyConfig>()?;
    Ok(config)
}

fn get_log_level(env_var: &str, default: Level) -> Level {
    match std::env::var(env_var) {
        Ok(level) => Level::from_str(&level).unwrap_or(default),
        Err(_) => default,
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let file_log_level = get_log_level("TPROXY_LOG_LEVEL_FILE", Level::INFO);
    let console_log_level = get_log_level("TPROXY_LOG_LEVEL_CONSOLE", Level::DEBUG);

    let file_appender = tracing_appender::rolling::RollingFileAppender::new(
        tracing_appender::rolling::Rotation::DAILY,
        "logs",
        "Translator.log",
    );

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(file_appender)
                .with_ansi(false)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_file(true)
                .with_line_number(true)
                .with_filter(tracing_subscriber::filter::LevelFilter::from_level(file_log_level))
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stdout)
                .with_filter(tracing_subscriber::filter::LevelFilter::from_level(console_log_level))
        )
        .init();

    let proxy_config = match process_cli_args() {
        Ok(p) => p,
        Err(e) => panic!("failed to load config: {}", e),
    };
    info!("Proxy Config: {:?}", &proxy_config);

    metrics::start_metrics_server();

    lib::TranslatorSv2::new(proxy_config).start().await;
}
