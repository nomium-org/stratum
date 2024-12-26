use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use once_cell::sync::Lazy;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerIdentity {
    pub worker_name: String,
    pub worker_id: String,
}

impl ToString for WorkerIdentity {
    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

pub static WORKER_NAME_STORE: Lazy<Arc<RwLock<HashMap<String, WorkerIdentity>>>> = 
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

pub fn store_worker(worker_name: String, worker_id: String) {
    let identity = WorkerIdentity {
        worker_name: worker_name.clone(),
        worker_id,
    };
    WORKER_NAME_STORE.write().insert(worker_name, identity);
}

pub fn get_worker_identity(worker_name: &str) -> Option<WorkerIdentity> {
    WORKER_NAME_STORE.read().get(worker_name).cloned()
}