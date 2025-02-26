#![allow(special_module_name)]

mod lib;
use ext_config::{Config, File, FileFormat, Environment};
pub use lib::{mining_pool::Configuration, status, PoolSv2};
use tracing::{error, debug};
use tracing_subscriber::prelude::*;
use dotenvy::dotenv;
use tracing::Level;
use std::str::FromStr;
use std::env;
use tokio::{signal, select};
use lib::wallet_rotation::{initialize_wallet_rotator, WalletConfig};

mod args {
    use std::path::PathBuf;

    #[derive(Debug)]
    pub struct Args {
        pub config_path: PathBuf,
    }

    enum ArgsState {
        Next,
        ExpectPath,
        Done,
    }

    enum ArgsResult {
        Config(PathBuf),
        None,
        Help(String),
    }

    impl Args {
        const DEFAULT_CONFIG_PATH: &'static str = "pool-config.toml";
        const HELP_MSG: &'static str =
            "Usage: -h/--help, -c/--config <path|default pool-config.toml>";

        pub fn from_args() -> Result<Self, String> {
            let cli_args = std::env::args();

            if cli_args.len() == 1 {
                println!("Using default config path: {}", Self::DEFAULT_CONFIG_PATH);
                println!("{}\n", Self::HELP_MSG);
            }

            let config_path = cli_args
                .scan(ArgsState::Next, |state, item| {
                    match std::mem::replace(state, ArgsState::Done) {
                        ArgsState::Next => match item.as_str() {
                            "-c" | "--config" => {
                                *state = ArgsState::ExpectPath;
                                Some(ArgsResult::None)
                            }
                            "-h" | "--help" => Some(ArgsResult::Help(Self::HELP_MSG.to_string())),
                            _ => {
                                *state = ArgsState::Next;

                                Some(ArgsResult::None)
                            }
                        },
                        ArgsState::ExpectPath => Some(ArgsResult::Config(PathBuf::from(item))),
                        ArgsState::Done => None,
                    }
                })
                .last();
            let config_path = match config_path {
                Some(ArgsResult::Config(p)) => p,
                Some(ArgsResult::Help(h)) => return Err(h),
                _ => PathBuf::from(Self::DEFAULT_CONFIG_PATH),
            };
            Ok(Self { config_path })
        }
    }
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
let file_log_level = get_log_level("POOL_LOG_LEVEL_FILE", Level::INFO);
let console_log_level = get_log_level("POOL_LOG_LEVEL_CONSOLE", Level::DEBUG);

let file_appender = tracing_appender::rolling::RollingFileAppender::new(
    tracing_appender::rolling::Rotation::DAILY,
    "logs",
    "Pool.log",
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

    let args = match args::Args::from_args() {
        Ok(cfg) => cfg,
        Err(help) => {
            error!("{}", help);
            return;
        }
    };

    let config_path = args.config_path.to_str().expect("Invalid config path");

    if let Ok(env_tp_address) = env::var("POOL__TP_ADDRESS") {
        debug!("Found POOL_TP_ADDRESS in environment: {}", env_tp_address);
    }
    // Load config
    let mut config: Configuration = match Config::builder()
        .add_source(File::new(config_path, FileFormat::Toml))
        .add_source(Environment::with_prefix("POOL").separator("__"))
        .build()
    {
        Ok(settings) => match settings.try_deserialize::<Configuration>() {
            Ok(c) => {
                debug!("Configuration loaded successfully");
                debug!("TP Address: {}", c.tp_address);
                debug!("Full config: {:?}", c);
                c
            },
            Err(e) => {
                error!("Failed to deserialize config: {}", e);
                return;
            }
        },
        Err(e) => {
            error!("Failed to build config: {}", e);
            return;
        }
    };

    match (
        env::var("POOL__COINBASE_OUTPUTS_0_OUTPUT_SCRIPT_TYPE"),
        env::var("POOL__COINBASE_OUTPUTS_0_OUTPUT_SCRIPT_VALUE"),
    ) {
        (Ok(output_script_type), Ok(output_script_value)) => {
            match config.coinbase_outputs.get_mut(0) {
                Some(output) => {
                    output.set_output_script_type(output_script_type);
                    output.set_output_script_value(output_script_value);
                    debug!("Overridden coinbase output: {:?}", output);
                }
                None => error!("coinbase_outputs is empty, cannot override values"),
            }
            debug!("Full config after override: {:?}", config);
        }
        _ => { /*  или ничего :) */ }
    }

    // ротатор --

    let mut wallet_configs = Vec::new();
    
    if let (Ok(type1), Ok(value1)) = (
        env::var("POOL__COINBASE_OUTPUTS_0_OUTPUT_SCRIPT_TYPE"),
        env::var("POOL__COINBASE_OUTPUTS_0_OUTPUT_SCRIPT_VALUE")
    ) {
        wallet_configs.push(WalletConfig {
            output_script_type: type1,
            output_script_value: value1,
        });
    } else if let Some(output) = config.coinbase_outputs.get(0) {
        wallet_configs.push(WalletConfig {
            output_script_type: output.get_output_script_type().clone(),
            output_script_value: output.get_output_script_value().clone(),
        });
    }
    
    if let (Ok(type2), Ok(value2)) = (
        env::var("POOL__COINBASE_OUTPUTS_1_OUTPUT_SCRIPT_TYPE"),
        env::var("POOL__COINBASE_OUTPUTS_1_OUTPUT_SCRIPT_VALUE")
    ) {
        wallet_configs.push(WalletConfig {
            output_script_type: type2,
            output_script_value: value2,
        });
    } else if let Some(output) = config.coinbase_outputs.get(1) {
        wallet_configs.push(WalletConfig {
            output_script_type: output.get_output_script_type().clone(),
            output_script_value: output.get_output_script_value().clone(),
        });
    }
    
    if wallet_configs.is_empty() {
        error!("No wallet configurations found!");
        return;
    }
    
    initialize_wallet_rotator(wallet_configs);

    // -- ротатор

    let pool = PoolSv2::new(config);

    select! {
        _ = signal::ctrl_c() => {
            error!("Received Ctrl+C signal, starting graceful shutdown");
        }
        result = pool.start() => {
            if let Err(e) = result {
                error!("Pool error: {:?}", e);
            }
        }
    }

    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
}
