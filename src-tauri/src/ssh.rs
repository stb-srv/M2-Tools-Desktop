use russh::client::{connect, Config, Handle, Handler};
use russh::keys::PublicKey;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
}

struct AcceptAllHandler;

impl Handler for AcceptAllHandler {
    type Error = russh::Error;

    async fn check_server_key(&mut self, _server_public_key: &PublicKey) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

async fn open_session(config: &SshConfig) -> Result<Handle<AcceptAllHandler>, String> {
    let ssh_config = Arc::new(Config::default());
    connect(
        ssh_config,
        (config.host.as_str(), config.port),
        AcceptAllHandler,
    )
    .await
    .map_err(|e| e.to_string())
}

pub async fn test_connection(config: &SshConfig, password: &str) -> Result<(), String> {
    let mut session = open_session(config).await?;
    let authenticated = session
        .authenticate_password(&config.username, password)
        .await
        .map_err(|e| e.to_string())?;
    if authenticated.success() {
        Ok(())
    } else {
        Err("Authentifizierung fehlgeschlagen".into())
    }
}

pub async fn run_command(
    config: &SshConfig,
    password: &str,
    command: &str,
) -> Result<String, String> {
    let mut session = open_session(config).await?;
    let authenticated = session
        .authenticate_password(&config.username, password)
        .await
        .map_err(|e| e.to_string())?;
    if !authenticated.success() {
        return Err("Authentifizierung fehlgeschlagen".into());
    }

    let mut channel = session.channel_open_session().await.map_err(|e| e.to_string())?;
    channel.exec(true, command).await.map_err(|e| e.to_string())?;

    let mut output = Vec::new();
    while let Some(msg) = channel.wait().await {
        if let russh::ChannelMsg::Data { ref data } = msg {
            output.extend_from_slice(data);
        }
    }

    Ok(String::from_utf8_lossy(&output).to_string())
}
