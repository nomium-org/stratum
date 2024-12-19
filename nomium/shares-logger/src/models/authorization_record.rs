use clickhouse::Row;
use serde::Serialize;

#[derive(Debug, Clone, Row, Serialize)]
pub struct ClickhouseAuthRecord {
    pub account_name: String,
    pub worker_number: u32,
    pub is_success: bool,
    pub worker_id: String,
    pub worker_name: String,
    pub user_id: String,
    pub account_id: String,
}

impl ClickhouseAuthRecord {
    pub fn new(
        account_name: String,
        worker_number: u32,
        is_success: bool,
        worker_id: String,
        worker_name: String,
        user_id: String,
        account_id: String,
    ) -> Self {
        Self {
            account_name,
            worker_number,
            is_success,
            worker_id,
            worker_name,
            user_id,
            account_id,
        }
    }
}