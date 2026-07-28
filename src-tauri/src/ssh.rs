use russh::client::{connect, Config, Handle, Handler, KeyboardInteractiveAuthResponse};
use russh::keys::{load_secret_key, PrivateKeyWithHashAlg, PublicKey};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SshAuth {
    #[serde(rename = "password")]
    Password { password: String },
    #[serde(rename = "private_key")]
    PrivateKey {
        path: String,
        passphrase: Option<String>,
    },
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

// Some servers (this FreeBSD/PAM setup among them) only advertise
// "publickey,keyboard-interactive" and reject the plain SSH "password"
// method outright, even though the login is conceptually just a password.
// Fall back to keyboard-interactive, answering every text prompt with the
// given password (matches how ssh/plink behave against the same server).
async fn authenticate_with_password(
    session: &mut Handle<AcceptAllHandler>,
    username: &str,
    password: &str,
) -> Result<bool, String> {
    let direct = session
        .authenticate_password(username, password)
        .await
        .map_err(|e| e.to_string())?;
    if direct.success() {
        return Ok(true);
    }

    let mut response = session
        .authenticate_keyboard_interactive_start(username, None)
        .await
        .map_err(|e| e.to_string())?;

    loop {
        match response {
            KeyboardInteractiveAuthResponse::Success => return Ok(true),
            KeyboardInteractiveAuthResponse::Failure { .. } => return Ok(false),
            KeyboardInteractiveAuthResponse::InfoRequest { prompts, .. } => {
                let answers = vec![password.to_string(); prompts.len()];
                response = session
                    .authenticate_keyboard_interactive_respond(answers)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
    }
}

async fn authenticate(
    session: &mut Handle<AcceptAllHandler>,
    username: &str,
    auth: &SshAuth,
) -> Result<bool, String> {
    match auth {
        SshAuth::Password { password } => {
            authenticate_with_password(session, username, password).await
        }
        SshAuth::PrivateKey { path, passphrase } => {
            let key = load_secret_key(path, passphrase.as_deref())
                .map_err(|e| format!("Privater Schlüssel konnte nicht geladen werden: {e}"))?;
            let result = session
                .authenticate_publickey(username, PrivateKeyWithHashAlg::new(Arc::new(key), None))
                .await
                .map_err(|e| e.to_string())?;
            Ok(result.success())
        }
    }
}

pub async fn test_connection(config: &SshConfig, auth: &SshAuth) -> Result<(), String> {
    let mut session = open_session(config).await?;
    if authenticate(&mut session, &config.username, auth).await? {
        Ok(())
    } else {
        Err("Authentifizierung fehlgeschlagen".into())
    }
}

pub struct CommandResult {
    pub output: String,
    pub exit_status: Option<u32>,
}

/// Runs a command and reports output as it arrives. Server scripts print
/// progress over several seconds (the start action sleeps between channels),
/// so collecting everything first would leave the UI blank until it finished.
pub async fn run_command_streaming<F>(
    config: &SshConfig,
    auth: &SshAuth,
    command: &str,
    mut on_output: F,
) -> Result<CommandResult, String>
where
    F: FnMut(&str),
{
    let mut session = open_session(config).await?;
    if !authenticate(&mut session, &config.username, auth).await? {
        return Err("Authentifizierung fehlgeschlagen".into());
    }

    let mut channel = session
        .channel_open_session()
        .await
        .map_err(|e| e.to_string())?;
    channel.exec(true, command).await.map_err(|e| e.to_string())?;

    let mut output = String::new();
    let mut exit_status = None;

    while let Some(msg) = channel.wait().await {
        match msg {
            russh::ChannelMsg::Data { ref data }
            | russh::ChannelMsg::ExtendedData { ref data, .. } => {
                let chunk = String::from_utf8_lossy(data);
                on_output(&chunk);
                output.push_str(&chunk);
            }
            russh::ChannelMsg::ExitStatus { exit_status: code } => {
                exit_status = Some(code);
            }
            _ => {}
        }
    }

    Ok(CommandResult {
        output,
        exit_status,
    })
}
