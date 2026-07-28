use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MysqlConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub database: Option<String>,
}

impl MysqlConfig {
    fn url(&self, password: &str) -> String {
        format!(
            "mysql://{}:{}@{}:{}/{}",
            self.username,
            password,
            self.host,
            self.port,
            self.database.clone().unwrap_or_default()
        )
    }
}

pub async fn test_connection(config: &MysqlConfig, password: &str) -> Result<(), String> {
    MySqlPoolOptions::new()
        .max_connections(1)
        .connect(&config.url(password))
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub async fn connect(config: &MysqlConfig, password: &str) -> Result<MySqlPool, String> {
    MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&config.url(password))
        .await
        .map_err(|e| e.to_string())
}
