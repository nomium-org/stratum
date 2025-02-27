use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tracing::{debug, info};
use std::env;
use crate::Configuration; 

pub struct WalletRotator {
    current_wallet_index: AtomicUsize,
    wallets: Vec<WalletConfig>,
}

pub struct WalletConfig {
    pub output_script_type: String,
    pub output_script_value: String,
}

impl WalletRotator {
    pub fn new(wallets: Vec<WalletConfig>) -> Arc<Self> {
        if wallets.is_empty() {
            panic!("Wallet rotator requires at least one wallet configuration");
        }
        
        info!("Initialized wallet rotator with {} wallets", wallets.len());
        for (i, wallet) in wallets.iter().enumerate() {
            debug!("Wallet {}: {} {}", i, wallet.output_script_type, wallet.output_script_value);
        }
        
        Arc::new(Self {
            current_wallet_index: AtomicUsize::new(0),
            wallets,
        })
    }
    
    pub fn rotate_wallet(&self) -> WalletConfig {
        let current = self.current_wallet_index.load(Ordering::Relaxed);
        let next = (current + 1) % self.wallets.len();
        self.current_wallet_index.store(next, Ordering::Relaxed);
        
        info!(
            "Rotating wallet from {} to {}", 
            current,
            next
        );
        info!(
            "New active wallet: Type={}, PubKey={}", 
            self.wallets[next].output_script_type,
            self.wallets[next].output_script_value
        );
        
        WalletConfig {
            output_script_type: self.wallets[next].output_script_type.clone(),
            output_script_value: self.wallets[next].output_script_value.clone(),
        }
    }
}

use std::sync::OnceLock;
static WALLET_ROTATOR: OnceLock<Arc<WalletRotator>> = OnceLock::new();

pub fn get_wallet_config(index: usize, config: &Configuration) -> Option<WalletConfig> {
    let env_result = (
        env::var(format!("POOL__COINBASE_OUTPUTS_{}_OUTPUT_SCRIPT_TYPE", index)),
        env::var(format!("POOL__COINBASE_OUTPUTS_{}_OUTPUT_SCRIPT_VALUE", index))
    );

    match env_result {
        (Ok(type_), Ok(value)) => Some(WalletConfig {
            output_script_type: type_,
            output_script_value: value,
        }),
        _ => config.coinbase_outputs.get(index).map(|output| WalletConfig {
            output_script_type: output.get_output_script_type().clone(),
            output_script_value: output.get_output_script_value().clone(),
        })
    }
}

pub fn initialize_wallet_rotator(wallets: Vec<WalletConfig>) {
    let _ = WALLET_ROTATOR.set(WalletRotator::new(wallets));
}

pub fn get_wallet_rotator() -> Arc<WalletRotator> {
    WALLET_ROTATOR.get().expect("Wallet rotator not initialized").clone()
}