use dotenvy::dotenv;
use lazy_static::lazy_static;
use prometheus::{register_int_counter, register_int_gauge, Encoder, IntCounter, IntGauge};
use std::env;
use std::thread;
use tiny_http::{Header, Response, Server};
use tracing::error;

lazy_static! {

    // SHARES_REFUSED_TEST_ as SHRT_
    pub static ref REFUSED_SHARES_SUMMARY: IntCounter =
        register_int_counter!("refused_shares_summary",
            "shrt_fn_handle_submit_refused + bridge_on_new_share_rejected_total").unwrap();

    pub static ref SHRT_SAVE_SHARE_TO_VARDIFF: IntCounter =
        register_int_counter!("shrt_save_share_to_vardiff_total",
            "Total number of shares received if message is Submit Shares update difficulty management").unwrap();

    pub static ref SHRT_FN_HANDLE_SUBMIT_REFUSED: IntCounter =
        register_int_counter!("shrt_fn_handle_submit_refused",
            "Total number of shares refused in fn handle_submit").unwrap();

    pub static ref SHRT_FN_TRANSLATE_SUBMIT_REFUSED: IntCounter =
        register_int_counter!("shrt_fn_translate_submit_refused",
            "Total number of shares refused in fn translate_submit").unwrap();

    pub static ref SHRT_DIFFICULTY_TOO_LOW_IN_CHANNEL_FACTORY: IntCounter =
        register_int_counter!("shrt_difficulty_too_low_in_channel_factory",
            "Total number of shares refused in channel_factory cause difficulty_too_low").unwrap();

    pub static ref SHRT_JOB_ID_IN_CHANNEL_FACTORY: IntCounter =
        register_int_counter!("shrt_job_id_in_channel_factory",
            "Total number of shares refused in channel_factory cause job_id").unwrap();

    pub static ref SHRT_INVALID_COINBASE_IN_CHANNEL_FACTORY: IntCounter =
        register_int_counter!("shrt_invalid_coinbase_in_channel_factory",
            "Total number of shares refused in channel_factory cause invalid_coinbase").unwrap();

    pub static ref SHRT_NO_TEMPLATE_FOR_ID_IN_CHANNEL_FACTORY: IntCounter =
        register_int_counter!("shrt_no_template_for_id_in_channel_factory",
            "Total number of shares refused in channel_factory cause NoTemplateForId").unwrap();

    //

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

    pub static ref BRIDGE_ON_NEW_SHARE_REJECTED_TOTAL: IntCounter =
        register_int_counter!("bridge_on_new_share_rejected_total",
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

    // batch and channels
    pub static ref SHALOG_PRIMARY_CHANNEL_CURRENT: IntGauge =
            register_int_gauge!("shalog_primary_channel_current",
                "Current number of shares in primary channel").unwrap();

    pub static ref SHALOG_BACKUP_CHANNEL_CURRENT: IntGauge =
        register_int_gauge!("shalog_backup_channel_current",
            "Current number of shares in backup channel").unwrap();

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
