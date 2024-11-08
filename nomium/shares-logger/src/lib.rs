use log::info;

pub fn hand_shake() {
    info!("!!! SHARES-LOGGER !!!");
}

pub fn log_share(hash: Vec<u8>) {
    info!("Share hash: {:?}", hex::encode(hash));
}