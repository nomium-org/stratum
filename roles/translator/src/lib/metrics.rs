use lazy_static::lazy_static;
use prometheus::{register_int_counter, register_int_gauge, IntCounter, IntGauge, Encoder};
use std::thread;
use tiny_http::{Server, Response};

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

    pub static ref ACTIVE_CONNECTIONS: IntGauge =
        register_int_gauge!("mining_active_connections", 
            "Number of active miner connections").unwrap();
}

pub fn start_metrics_server() {
    thread::spawn(|| {
        let server = Server::http("0.0.0.0:9184").unwrap();
        
        for request in server.incoming_requests() {
            let encoder = prometheus::TextEncoder::new();
            let mut buffer = Vec::new();
            encoder.encode(&prometheus::gather(), &mut buffer).unwrap();
            
            let response = Response::from_data(buffer);
            let _ = request.respond(response);
        }
    });
}