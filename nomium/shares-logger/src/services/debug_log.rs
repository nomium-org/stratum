use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::path::Path;
use crate::ShareLog;
use std::sync::Mutex;
use lazy_static::lazy_static;
use log::{info, error};

lazy_static! {
    static ref FILE_HANDLES: Mutex<Vec<(String, std::fs::File)>> = Mutex::new(Vec::new());
}

pub fn log_share_hash(location: &str, share: &ShareLog) {
    let base_dir = Path::new("/home/ro/projects/Stratum-RedRock_Pool/nomium-stratum-rust-dev/nomium/shares-logger");
    let log_dir = base_dir.join("debug_logs");

    match create_dir_all(&log_dir) {
        Ok(_) => info!("Директория логов создана успешно: {:?}", log_dir),
        Err(e) => error!("Ошибка создания директории логов: {}", e),
    }

    let file_path = log_dir.join(format!("{}_hashes.log", location));
    let hash_hex = hex::encode(&share.hash);
    
    let mut handles = match FILE_HANDLES.lock() {
        Ok(guard) => guard,
        Err(e) => {
            error!("Ошибка блокировки FILE_HANDLES: {}", e);
            return;
        }
    };

    if !handles.iter().any(|(loc, _)| loc == location) {
        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
        {
            Ok(file) => {
                handles.push((location.to_string(), file));
            }
            Err(e) => {
                error!("Ошибка открытия файла {:?}: {}", file_path, e);
                return;
            }
        }
    }

    if let Some((_, file)) = handles.iter_mut().find(|(loc, _)| loc == location) {
        match writeln!(file, "{}", hash_hex) {
            Ok(_) => info!("Записан хэш: {}", hash_hex),
            Err(e) => error!("Ошибка записи хэша: {}", e),
        }
    }
}