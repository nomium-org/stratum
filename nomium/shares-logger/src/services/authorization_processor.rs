use crate::models::AuthorizationLog;
use tokio::sync::mpsc;
use log::info;

pub struct AuthorizationProcessor {
    receiver: mpsc::Receiver<AuthorizationLog>,
}

impl AuthorizationProcessor {
    pub fn new(receiver: mpsc::Receiver<AuthorizationLog>) -> Self {
        Self { receiver }
    }

    pub async fn run(&mut self) {
        while let Some(auth_log) = self.receiver.recv().await {
            info!(
                "Processing authorization: name = {}, password = {}",
                auth_log.name, auth_log.password
            );
        }
    }
}