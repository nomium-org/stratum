use once_cell::sync::Lazy;

pub struct ClickhouseConfig {
    pub url: String,
    pub database: String,
    pub username: String,
    pub password: String,
    pub batch_size: usize,
}

pub static CONFIG: Lazy<ClickhouseConfig> = Lazy::new(|| {
    ClickhouseConfig {
        url: "http://localhost:8123".to_string(),
        database: "mining".to_string(),
        username: "default".to_string(),
        password: "5555".to_string(),
        batch_size: 2,
    }
});