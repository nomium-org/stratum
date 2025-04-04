#![allow(special_module_name)]
mod args;
mod lib;

use crate::lib::metrics;
use args::Args;
use dotenvy::dotenv;
use error::{Error, ProxyResult};
pub use lib::{downstream_sv1, error, proxy, proxy_config, status, upstream_sv2};
use proxy_config::ProxyConfig;
use std::str::FromStr;
use tracing::Level;
use tracing_subscriber::prelude::*;

use ext_config::{Config, File, FileFormat};

use tracing::{error, info};

/// Process CLI args, if any.
fn process_cli_args<'a>() -> ProxyResult<'a, ProxyConfig> {
    let args = Args::from_args().map_err(|help| {
        error!("{}", help);
        Error::BadCliArgs
    })?;

    let config_path = args.config_path.to_str().ok_or_else(|| {
        error!("Invalid configuration path.");
        Error::BadCliArgs
    })?;

    let settings = Config::builder()
        .add_source(File::new(config_path, FileFormat::Toml))
        .build()?;

    let config: ProxyConfig = settings.try_deserialize()?;

    let config = config.apply_env_overrides();

    Ok(config)
}

fn get_log_level(env_var: &str, default: Level) -> Level {
    match std::env::var(env_var) {
        Ok(level) => Level::from_str(&level).unwrap_or(default),
        Err(_) => default,
    }
}

fn should_show_shares_logs() -> bool {
    match std::env::var("TPROXY__LOG_TARGET_SHARES_SHOW") {
        Ok(val) => val.to_lowercase() == "true",
        Err(_) => true,
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
                .with_filter(tracing_subscriber::filter::LevelFilter::from_level(
                    file_log_level,
                )),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stdout)
                .with_filter(tracing_subscriber::filter::LevelFilter::from_level(
                    console_log_level,
                ))
                .with_filter(
                    tracing_subscriber::filter::filter_fn(move |metadata| {
                        if metadata.target().starts_with("shares") {
                            should_show_shares_logs()
                        } else {
                            metadata.level() <= &file_log_level
                        }
                    })
                )
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
