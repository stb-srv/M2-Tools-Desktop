use serde::{Deserialize, Serialize};
use sqlx::mysql::{MySqlConnectOptions, MySqlPoolOptions, MySqlSslMode};
use sqlx::MySqlPool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MysqlConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub database: Option<String>,
}

impl MysqlConfig {
    /// Verbindungsoptionen ohne URL-String: Benutzername/Passwort/Host dürfen
    /// beliebige Zeichen (`@`, `/`, `:` ...) enthalten, ohne dass etwas
    /// escaped werden müsste oder das Passwort in einer URL landet.
    fn options(&self, password: &str, ssl_mode: MySqlSslMode) -> MySqlConnectOptions {
        let mut opts = MySqlConnectOptions::new()
            .host(&self.host)
            .port(self.port)
            .username(&self.username)
            .password(password)
            .ssl_mode(ssl_mode);
        if let Some(db) = self.database.as_deref().filter(|d| !d.is_empty()) {
            opts = opts.database(db);
        }
        opts
    }
}

async fn pool_with(config: &MysqlConfig, password: &str, max: u32) -> Result<MySqlPool, String> {
    // TLS wird verwendet, wenn der Server es anbietet (Preferred). Scheitert
    // schon der TLS-Handshake (z. B. alter Server nur mit TLS 1.0), wird einmal
    // unverschlüsselt wie bisher verbunden, damit bestehende Setups weiter laufen.
    let build = |mode| MySqlPoolOptions::new().max_connections(max).connect_with(config.options(password, mode));
    match build(MySqlSslMode::Preferred).await {
        Err(sqlx::Error::Tls(_)) => build(MySqlSslMode::Disabled).await,
        other => other,
    }
    .map_err(|e| e.to_string())
}

pub async fn test_connection(config: &MysqlConfig, password: &str) -> Result<(), String> {
    pool_with(config, password, 1).await.map(|_| ())
}

pub async fn connect(config: &MysqlConfig, password: &str) -> Result<MySqlPool, String> {
    pool_with(config, password, 5).await
}
