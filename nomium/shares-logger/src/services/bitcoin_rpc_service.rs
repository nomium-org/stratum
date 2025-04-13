use crate::config::SETTINGS;
use log::{error, info};
use rpc_sv2::mini_rpc_client::{Auth, MiniRpcClient, RpcError};

pub struct BitcoinRpcService {
    client: MiniRpcClient,
}

impl BitcoinRpcService {
    pub fn new() -> Result<Self, RpcError> {
        let settings = &SETTINGS.bitcoin_rpc;
        if settings.url.is_empty() || settings.port == 0 || settings.user.is_empty() || settings.password.is_empty() {
            return Err(RpcError::Other(
                "Bitcoin RPC settings are not properly configured. \
                Make sure to set SHARES_LOGGER__BITCOIN_RPC__URL, \
                SHARES_LOGGER__BITCOIN_RPC__PORT, \
                SHARES_LOGGER__BITCOIN_RPC__USER, \
                SHARES_LOGGER__BITCOIN_RPC__PASSWORD environment variables.".to_string()
            ));
        }
        let url = format!("{}:{}", settings.url, settings.port);
        let auth = Auth::new(settings.user.clone(), settings.password.clone());
        info!(target: "shares", "Initializing Bitcoin RPC client with URL: {}", url);
        let client = MiniRpcClient::new(url, auth);
        Ok(Self { client })
    }

    pub async fn is_block_in_blockchain(&self, block_hash: &str) -> Result<bool, RpcError> {
        info!(target: "shares", "Checking existence of block {} in blockchain", block_hash);
        info!(target: "shares", "Using RPC connection: URL={}, User={}", 
              format!("{}:{}", SETTINGS.bitcoin_rpc.url, SETTINGS.bitcoin_rpc.port),
              SETTINGS.bitcoin_rpc.user);

        if block_hash.len() != 64 {
            error!(target: "shares", "Invalid block hash format: {}, expected 32-byte hex string (64 characters)", block_hash);
            return Err(RpcError::Other(format!("Invalid block hash format: {}", block_hash)));
        }
        info!(target: "shares", "Sending RPC request getblockheader for block: {}", block_hash);
        match self.client.send_json_rpc_request("getblockheader", serde_json::json!([block_hash])).await {
            Ok(response) => {
                info!(target: "shares", "Received response from RPC server for block {}", block_hash);
                info!(target: "shares", "Full response: {}", response);
                match serde_json::from_str::<serde_json::Value>(&response) {
                    Ok(parsed) => {
                        if parsed.get("result").is_some() {
                            info!(target: "shares", "Block {} confirmed in blockchain", block_hash);

                            if let Some(result) = parsed.get("result") {
                                if let Some(height) = result.get("height") {
                                    info!(target: "shares", "Block {} height: {}", block_hash, height);
                                }
                                if let Some(confirmations) = result.get("confirmations") {
                                    info!(target: "shares", "Number of confirmations for block {}: {}", 
                                         block_hash, confirmations);
                                }
                            }
                            Ok(true)
                        } else if let Some(error) = parsed.get("error") {
                            let error_code = error.get("code")
                                .and_then(|c| c.as_i64())
                                .unwrap_or(0);
                            let error_message = error.get("message")
                                .and_then(|m| m.as_str())
                                .unwrap_or("Unknown error");
                            info!(target: "shares", "RPC returned error: code={}, message='{}'", 
                                   error_code, error_message);
                            // Code -5 means "Block not found"
                            if error_code == -5 || error_message.contains("Block not found") {
                                info!(target: "shares", "Block {} not found in blockchain", block_hash);
                                return Ok(false);
                            }
                            error!(target: "shares", "RPC error when checking block {}: code={}, message='{}'", 
                                  block_hash, error_code, error_message);
                            Err(RpcError::Other(
                                format!("RPC error: code={}, message='{}'", error_code, error_message)
                            ))
                        } else {
                            error!(target: "shares", "Invalid RPC response format: neither result nor error present");
                            Err(RpcError::Other("Invalid RPC response format".into()))
                        }
                    },
                    Err(e) => {
                        error!(target: "shares", "Failed to deserialize RPC response: {}", e);
                        error!(target: "shares", "Raw response: {}", response);
                        Err(RpcError::Deserialization(format!("JSON parsing error: {}", e)))
                    }
                }
            },
            Err(e) => {
                error!(target: "shares", "Error executing RPC request for block {}: {:?}", block_hash, e);
                match &e {
                    RpcError::JsonRpc(ref err) => {
                        error!(target: "shares", "RPC error details: {:?}", err);
                        if let Some(error) = &err.error {
                            info!(target: "shares", "RPC returned error: code={}, message='{}'", 
                                   error.code, error.message);
                            if error.code == -5 || error.message.contains("Block not found") {
                                info!(target: "shares", "Block {} not found in blockchain", block_hash);
                                return Ok(false);
                            }
                        }
                    },
                    _ => {
                        error!(target: "shares", "Error details: {:?}", e);
                    }
                }
                Err(e)
            }
        }
    }
}
