use russh::client::{connect, Config, Handle, Handler, KeyboardInteractiveAuthResponse};
use russh::keys::{load_secret_key, PrivateKeyWithHashAlg, PublicKey};
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::OpenFlags;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

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

async fn open_sftp(config: &SshConfig, auth: &SshAuth) -> Result<SftpSession, String> {
    let mut session = open_session(config).await?;
    if !authenticate(&mut session, &config.username, auth).await? {
        return Err("Authentifizierung fehlgeschlagen".into());
    }
    let channel = session
        .channel_open_session()
        .await
        .map_err(|e| e.to_string())?;
    channel
        .request_subsystem(true, "sftp")
        .await
        .map_err(|e| e.to_string())?;
    SftpSession::new(channel.into_stream())
        .await
        .map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteEntry {
    pub name: String,
    pub is_dir: bool,
}

/// Lists one directory level (not recursive) - used by the Regen-Datei-Editor
/// and Backup-Browser to let the user browse into `share/data/dungeon/...`
/// and `*/m2manager_backups` without hardcoding every possible layout.
pub async fn list_remote_dir(
    config: &SshConfig,
    auth: &SshAuth,
    path: &str,
) -> Result<Vec<RemoteEntry>, String> {
    let sftp = open_sftp(config, auth).await?;
    let entries = sftp
        .read_dir(path)
        .await
        .map_err(|e| format!("Ordner konnte nicht gelesen werden ({path}): {e}"))?;

    let mut result: Vec<RemoteEntry> = entries
        .map(|entry| RemoteEntry {
            name: entry.file_name(),
            is_dir: entry.file_type().is_dir(),
        })
        .collect();
    result.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
    Ok(result)
}

/// Moves a remote file into the same `m2manager_backups` folder used by
/// `write_remote_file_with_backup`, without writing a replacement - used for
/// "delete" actions where we never want to actually destroy server data.
/// Returns the backup path, or `None` if the file didn't exist.
pub async fn delete_remote_file_with_backup(
    config: &SshConfig,
    auth: &SshAuth,
    path: &str,
) -> Result<Option<String>, String> {
    let sftp = open_sftp(config, auth).await?;

    if !sftp.try_exists(path).await.map_err(|e| e.to_string())? {
        return Ok(None);
    }

    let (dir, filename) = match path.rfind('/') {
        Some(idx) => (&path[..idx], &path[idx + 1..]),
        None => (".", path),
    };
    let backup_dir = format!("{dir}/m2manager_backups");
    if !sftp.try_exists(&backup_dir).await.unwrap_or(false) {
        let _ = sftp.create_dir(&backup_dir).await;
    }
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let dest = format!("{backup_dir}/{filename}.{timestamp}.deleted");
    sftp.rename(path, &dest)
        .await
        .map_err(|e| format!("Löschen (Backup-Verschiebung) fehlgeschlagen: {e}"))?;
    Ok(Some(dest))
}

/// Permanently removes a remote file - unlike `delete_remote_file_with_backup`,
/// no backup is made first. Used for DB dump files, which are themselves
/// already a backup/snapshot rather than a live server file, so renaming one
/// into another backup folder before deleting it would be pointless churn.
pub async fn delete_remote_file(config: &SshConfig, auth: &SshAuth, path: &str) -> Result<(), String> {
    let sftp = open_sftp(config, auth).await?;
    sftp.remove_file(path).await.map_err(|e| format!("Löschen fehlgeschlagen ({path}): {e}"))
}

// Quest/config source files on this server are a mix of plain UTF-8 and
// Windows-1252 (whatever the quest designer's editor happened to save as -
// e.g. `Biologie/Biochecker.lua` has raw cp1252 umlaut bytes, which aren't
// valid UTF-8 and used to make this fail outright). Try strict UTF-8 first
// so already-clean files decode byte-for-byte; only fall back to a cp1252
// decode (which - unlike UTF-8 - never fails, every byte maps to something)
// for files that actually need it.
fn decode_bytes(bytes: Vec<u8>) -> String {
    match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(e) => {
            let (text, _, _) = encoding_rs::WINDOWS_1252.decode(e.as_bytes());
            text.into_owned()
        }
    }
}

pub async fn read_remote_file(config: &SshConfig, auth: &SshAuth, path: &str) -> Result<String, String> {
    let sftp = open_sftp(config, auth).await?;
    let bytes = sftp
        .read(path)
        .await
        .map_err(|e| format!("Datei konnte nicht gelesen werden ({path}): {e}"))?;
    Ok(decode_bytes(bytes))
}

/// Like `read_remote_file`, but returns `None` instead of an error when the
/// file doesn't exist - used where a missing file is an expected, harmless
/// case (e.g. diffing a backup against a target that may have been deleted
/// since) rather than something to report as a failure.
pub async fn read_remote_file_if_exists(
    config: &SshConfig,
    auth: &SshAuth,
    path: &str,
) -> Result<Option<String>, String> {
    let sftp = open_sftp(config, auth).await?;
    if !sftp.try_exists(path).await.map_err(|e| e.to_string())? {
        return Ok(None);
    }
    let bytes = sftp
        .read(path)
        .await
        .map_err(|e| format!("Datei konnte nicht gelesen werden ({path}): {e}"))?;
    Ok(Some(decode_bytes(bytes)))
}

/// Opens one SFTP session and reads every path within it - used by the Quest
/// Builder's full-text search, which would otherwise pay a full SSH
/// handshake per file (each of the other `read_remote_file*` helpers opens
/// its own session, fine for single-file reads but far too slow across a
/// whole quest catalogue).
pub async fn read_remote_files(
    config: &SshConfig,
    auth: &SshAuth,
    paths: &[String],
) -> Result<Vec<Result<String, String>>, String> {
    let sftp = open_sftp(config, auth).await?;
    let mut results = Vec::with_capacity(paths.len());
    for path in paths {
        let result = sftp
            .read(path)
            .await
            .map(decode_bytes)
            .map_err(|e| format!("Datei konnte nicht gelesen werden ({path}): {e}"));
        results.push(result);
    }
    Ok(results)
}

/// Backs up the existing remote file (rename, no re-upload needed - the
/// contents never leave the server) before overwriting it, mirroring the
/// local backup pattern in `packtools.rs`. Returns the backup path, or
/// `None` if there was nothing to back up (first write to a new path).
pub async fn write_remote_file_with_backup(
    config: &SshConfig,
    auth: &SshAuth,
    path: &str,
    content: &str,
) -> Result<Option<String>, String> {
    let sftp = open_sftp(config, auth).await?;

    let mut backup_path = None;
    if sftp.try_exists(path).await.map_err(|e| e.to_string())? {
        let (dir, filename) = match path.rfind('/') {
            Some(idx) => (&path[..idx], &path[idx + 1..]),
            None => (".", path),
        };
        let backup_dir = format!("{dir}/m2manager_backups");
        if !sftp.try_exists(&backup_dir).await.unwrap_or(false) {
            let _ = sftp.create_dir(&backup_dir).await;
        }
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let dest = format!("{backup_dir}/{filename}.{timestamp}.bak");
        sftp.rename(path, &dest)
            .await
            .map_err(|e| format!("Backup fehlgeschlagen: {e}"))?;
        backup_path = Some(dest);
    }

    // sftp.write() only opens with WRITE (no CREATE), which would fail here
    // since the path above was just renamed away - open with CREATE
    // explicitly so this also works for a brand-new file.
    let mut file = sftp
        .open_with_flags(path, OpenFlags::CREATE | OpenFlags::TRUNCATE | OpenFlags::WRITE)
        .await
        .map_err(|e| format!("Datei konnte nicht geschrieben werden ({path}): {e}"))?;
    file.write_all(content.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    file.shutdown().await.map_err(|e| e.to_string())?;

    Ok(backup_path)
}
