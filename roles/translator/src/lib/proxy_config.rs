use key_utils::Secp256k1PublicKey;
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize, Clone)]
pub struct ProxyConfig {
    #[serde(default = "default_upstream_address")]
    pub upstream_address: String,
    #[serde(default = "default_upstream_port")]
    pub upstream_port: u16,
    #[serde(default = "default_upstream_pubkey")]
    pub upstream_authority_pubkey: Secp256k1PublicKey,
    #[serde(default = "default_downstream_address")]
    pub downstream_address: String,
    #[serde(default = "default_downstream_port")]
    pub downstream_port: u16,
    pub max_supported_version: u16,
    pub min_supported_version: u16,
    pub min_extranonce2_size: u16,
    #[serde(default = "default_downstream_config")]
    pub downstream_difficulty_config: DownstreamDifficultyConfig,
    #[serde(default = "default_upstream_config")]
    pub upstream_difficulty_config: UpstreamDifficultyConfig,
}

fn default_upstream_address() -> String {
    "127.0.0.1".to_string()
}

fn default_upstream_port() -> u16 {
    34254
}

fn default_upstream_pubkey() -> Secp256k1PublicKey {
    "9auqWEzQDVyd2oe1JVGFLMLHZtCo2FFqZwtKA5gd9xbuEu7PH72"
        .parse()
        .unwrap()
}

fn default_downstream_address() -> String {
    "0.0.0.0".to_string()
}

fn default_downstream_port() -> u16 {
    34255
}

fn default_downstream_config() -> DownstreamDifficultyConfig {
    DownstreamDifficultyConfig {
        min_individual_miner_hashrate: default_min_hashrate(),
        shares_per_minute: default_shares_per_minute(),
        submits_since_last_update: 0,
        timestamp_of_last_update: 0,
    }
}

fn default_upstream_config() -> UpstreamDifficultyConfig {
    UpstreamDifficultyConfig {
        channel_diff_update_interval: default_channel_diff_update_interval(),
        channel_nominal_hashrate: default_channel_nominal_hashrate(),
        timestamp_of_last_update: 0,
        should_aggregate: false,
    }
}

impl ProxyConfig {
    pub fn apply_env_overrides(mut self) -> Self {
        if let Ok(addr) = env::var("UPSTREAM_ADDRESS") {
            self.upstream_address = addr;
        }
        if let Ok(port) = env::var("UPSTREAM_PORT") {
            if let Ok(port) = port.parse() {
                self.upstream_port = port;
            }
        }
        if let Ok(pubkey) = env::var("UPSTREAM_AUTHORITY_PUBKEY") {
            if let Ok(key) = pubkey.parse() {
                self.upstream_authority_pubkey = key;
            }
        }
        if let Ok(addr) = env::var("DOWNSTREAM_ADDRESS") {
            self.downstream_address = addr;
        }
        if let Ok(port) = env::var("DOWNSTREAM_PORT") {
            if let Ok(port) = port.parse() {
                self.downstream_port = port;
            }
        }

        if let Ok(hashrate) = env::var("MIN_MINER_HASHRATE") {
            if let Ok(hashrate) = hashrate.parse() {
                self.downstream_difficulty_config
                    .min_individual_miner_hashrate = hashrate;
            }
        }
        if let Ok(shares) = env::var("SHARES_PER_MINUTE") {
            if let Ok(shares) = shares.parse() {
                self.downstream_difficulty_config.shares_per_minute = shares;
            }
        }

        if let Ok(interval) = env::var("CHANNEL_DIFF_UPDATE_INTERVAL") {
            if let Ok(interval) = interval.parse() {
                self.upstream_difficulty_config.channel_diff_update_interval = interval;
            }
        }
        if let Ok(hashrate) = env::var("CHANNEL_NOMINAL_HASHRATE") {
            if let Ok(hashrate) = hashrate.parse() {
                self.upstream_difficulty_config.channel_nominal_hashrate = hashrate;
            }
        }

        self
    }

    pub fn new(
        upstream: UpstreamConfig,
        downstream: DownstreamConfig,
        max_supported_version: u16,
        min_supported_version: u16,
        min_extranonce2_size: u16,
    ) -> Self {
        Self {
            upstream_address: upstream.address,
            upstream_port: upstream.port,
            upstream_authority_pubkey: upstream.authority_pubkey,
            downstream_address: downstream.address,
            downstream_port: downstream.port,
            max_supported_version,
            min_supported_version,
            min_extranonce2_size,
            downstream_difficulty_config: downstream.difficulty_config,
            upstream_difficulty_config: upstream.difficulty_config,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct DownstreamDifficultyConfig {
    #[serde(default = "default_min_hashrate")]
    pub min_individual_miner_hashrate: f32,
    #[serde(default = "default_shares_per_minute")]
    pub shares_per_minute: f32,
    #[serde(default)]
    pub submits_since_last_update: u32,
    #[serde(default)]
    pub timestamp_of_last_update: u64,
}

fn default_min_hashrate() -> f32 {
    15_000_000.0
}

fn default_shares_per_minute() -> f32 {
    10.0
}

impl DownstreamDifficultyConfig {
    pub fn new(
        min_individual_miner_hashrate: f32,
        shares_per_minute: f32,
        submits_since_last_update: u32,
        timestamp_of_last_update: u64,
    ) -> Self {
        Self {
            min_individual_miner_hashrate,
            shares_per_minute,
            submits_since_last_update,
            timestamp_of_last_update,
        }
    }
}

impl PartialEq for DownstreamDifficultyConfig {
    fn eq(&self, other: &Self) -> bool {
        other.min_individual_miner_hashrate.round() as u32
            == self.min_individual_miner_hashrate.round() as u32
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct UpstreamDifficultyConfig {
    #[serde(default = "default_channel_diff_update_interval")]
    pub channel_diff_update_interval: u32,
    #[serde(default = "default_channel_nominal_hashrate")]
    pub channel_nominal_hashrate: f32,
    #[serde(default)]
    pub timestamp_of_last_update: u64,
    #[serde(default)]
    pub should_aggregate: bool,
}

fn default_channel_diff_update_interval() -> u32 {
    60
}

fn default_channel_nominal_hashrate() -> f32 {
    15_000_000.0
}

impl UpstreamDifficultyConfig {
    pub fn new(
        channel_diff_update_interval: u32,
        channel_nominal_hashrate: f32,
        timestamp_of_last_update: u64,
        should_aggregate: bool,
    ) -> Self {
        Self {
            channel_diff_update_interval,
            channel_nominal_hashrate,
            timestamp_of_last_update,
            should_aggregate,
        }
    }
}

pub struct UpstreamConfig {
    pub address: String,
    pub port: u16,
    pub authority_pubkey: Secp256k1PublicKey,
    pub difficulty_config: UpstreamDifficultyConfig,
}

impl UpstreamConfig {
    pub fn new(
        address: String,
        port: u16,
        authority_pubkey: Secp256k1PublicKey,
        difficulty_config: UpstreamDifficultyConfig,
    ) -> Self {
        Self {
            address,
            port,
            authority_pubkey,
            difficulty_config,
        }
    }
}

pub struct DownstreamConfig {
    pub address: String,
    pub port: u16,
    pub difficulty_config: DownstreamDifficultyConfig,
}

impl DownstreamConfig {
    pub fn new(address: String, port: u16, difficulty_config: DownstreamDifficultyConfig) -> Self {
        Self {
            address,
            port,
            difficulty_config,
        }
    }
}
