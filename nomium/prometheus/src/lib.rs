use lazy_static::lazy_static;
use prometheus::{register_int_counter, register_int_gauge, IntCounter, IntGauge, Encoder};
use std::thread;
use tiny_http::{Server, Response, Header};
use dotenvy::dotenv;
use std::env;
use tracing::error;

lazy_static! {
    pub static ref SHARES_RECEIVED: IntCounter =
        register_int_counter!("mining_shares_received_total", 
            "Total number of shares received from miners before any validation").unwrap();

    pub static ref SHARES_VALID_JOBID: IntCounter = 
        register_int_counter!("mining_shares_valid_jobid_total", 
            "Number of shares with valid job ID").unwrap();

    pub static ref SHARES_UPSTREAM_TARGET_MEET: IntCounter =
        register_int_counter!("mining_shares_upstream_target_meet_total", 
            "Shares that meet upstream pool target").unwrap();
        
    pub static ref SHARES_DOWNSTREAM_TARGET_MEET: IntCounter =
        register_int_counter!("mining_shares_downstream_target_meet_total", 
            "Shares that meet downstream (miner) target").unwrap();

    pub static ref TPROXY_SHARES_REJECTED_TOTAL: IntCounter =
        register_int_counter!("tproxy_shares_rejected_total", 
            "Total number of rejected shares").unwrap();

    pub static ref ACTIVE_CONNECTIONS: IntGauge =
        register_int_gauge!("mining_active_connections", 
            "Number of active miner connections").unwrap();

    pub static ref CONNECTION_ATTEMPTS: IntCounter =
        register_int_counter!("mining_connection_attempts_total", 
            "Total number of connection attempts").unwrap();
                    
    pub static ref CONNECTION_FAILURES: IntCounter =
        register_int_counter!("mining_connection_failures_total", 
            "Number of failed connection attempts").unwrap();
        
    pub static ref CONNECTION_AUTH_FAILURES: IntCounter = 
        register_int_counter!("mining_auth_failures_total",
            "Number of authentication failures").unwrap();
                
    pub static ref CONNECTION_TIMEOUT_FAILURES: IntCounter =
        register_int_counter!("mining_timeout_failures_total",
            "Number of connection timeouts").unwrap();

    pub static ref CHFACT_SHARES_LOGGED_TOTAL: IntCounter =
        register_int_counter!("chfact_shares_logged_total", 
            "Total number of shares sent to logger").unwrap();

    pub static ref SHALOG_SHARES_RECEIVED_TOTAL: IntCounter =
        register_int_counter!("shalog_shares_received_total", 
            "Total number of shares received by shares-logger").unwrap();
        
    pub static ref SHALOG_PRIMARY_CHANNEL_SHARES_TOTAL: IntCounter =
        register_int_counter!("shalog_primary_channel_shares_total", 
            "Total number of shares sent through primary channel").unwrap();
        
    pub static ref SHALOG_BACKUP_CHANNEL_SHARES_TOTAL: IntCounter =
        register_int_counter!("shalog_backup_channel_shares_total", 
            "Total number of shares sent through backup channel").unwrap();
        
    pub static ref SHALOG_PRIMARY_TRY_STORED_TOTAL: IntCounter =
        register_int_counter!("shalog_primary_try_stored_total", 
            "Total number of shares stored from primary channel").unwrap();
        
    pub static ref SHALOG_BACKUP_TRY_STORED_TOTAL: IntCounter =
        register_int_counter!("shalog_backup_try_stored_total", 
            "Total number of shares stored from backup channel").unwrap();

    pub static ref SHALOG_PRIMARY_STORE_FAILED_TOTAL: IntCounter =
        register_int_counter!("shalog_primary_store_failed_total", 
            "Total number of shares failed to store from primary channel").unwrap();
        
    pub static ref SHALOG_BACKUP_STORE_FAILED_TOTAL: IntCounter =
        register_int_counter!("shalog_backup_store_failed_total", 
            "Total number of shares failed to store from backup channel").unwrap();

}

pub fn start_metrics_server() {
    thread::spawn(|| {
        dotenv().ok();

        let metrics_ip = env::var("TPROXY_METRICS_IP").unwrap_or_else(|_| "0.0.0.0".to_string());
        let metrics_port = env::var("TPROXY_METRICS_PORT").unwrap_or_else(|_| "9184".to_string());

        let metrics_address = format!("{}:{}", metrics_ip, metrics_port);

        match Server::http(&metrics_address) {
            Ok(server) => {
                for request in server.incoming_requests() {
                    let encoder = prometheus::TextEncoder::new();
                    let mut buffer = Vec::new();

                    if let Err(e) = encoder.encode(&prometheus::gather(), &mut buffer) {
                        error!("Failed to encode metrics: {}", e);
                        continue;
                    }

                    let content_type = Header::from_bytes("Content-Type", "text/plain").unwrap();
                    let response = Response::from_data(buffer).with_header(content_type);

                    if let Err(e) = request.respond(response) {
                        error!("Failed to send metrics response: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Failed to start metrics server: {}", e);
            }
        }
    });
}