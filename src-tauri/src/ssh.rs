use russh::client::{connect, Config, Handle, Handler, KeyboardInteractiveAuthResponse};
use russh::keys::{load_secret_key, HashAlg, PrivateKeyWithHashAlg, PublicKey};
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::OpenFlags;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

// No SSH operation in this file had a timeout anywhere - a TCP-level stall
// (dropped packets, firewall, flaky network) makes `connect()`/the login
// handshake hang indefinitely with zero error surfaced, since there's
// nothing to time out and report. Real live report (2026-08-11): a
// multi-file write froze forever with no error message, because a single
// stalled connection among ~20 connect+login round-trips was enough to hang
// the entire operation. This bounds every
// connect/login attempt so a stall always turns into a clear, actionable
// error instead of an infinite spinner.
const SSH_TIMEOUT: Duration = Duration::from_secs(20);

/// Zeitlimit pro einzelner SFTP-Dateioperation (Lesen, Schreiben, Umbenennen ...).
const SFTP_OP_TIMEOUT: Duration = Duration::from_secs(60);

/// Gesamtlimit für Befehle, die `run_command_streaming` ohne eigene Angabe
/// startet. Großzügig, weil darüber auch Build-/Dump-/Deploy-Schritte laufen;
/// wer wirklich unbegrenzt warten will, nutzt `run_command_with_timeout(.., None, ..)`.
const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(30 * 60);

async fn read_bytes(sftp: &SftpSession, path: &str) -> Result<Vec<u8>, String> {
    sftp_op(&format!("Datei konnte nicht gelesen werden ({path})"), sftp.read(path)).await
}

async fn exists(sftp: &SftpSession, path: &str) -> Result<bool, String> {
    sftp_op(&format!("Prüfung fehlgeschlagen ({path})"), sftp.try_exists(path)).await
}

/// Sichert eine einzelne SFTP-Operation gegen Hänger ab.
async fn sftp_op<T, E: std::fmt::Display>(
    what: &str,
    fut: impl std::future::Future<Output = Result<T, E>>,
) -> Result<T, String> {
    match tokio::time::timeout(SFTP_OP_TIMEOUT, fut).await {
        Ok(Ok(v)) => Ok(v),
        Ok(Err(e)) => Err(format!("{what}: {e}")),
        Err(_) => Err(format!(
            "{what}: keine Antwort des Servers nach {}s (Zeitüberschreitung).",
            SFTP_OP_TIMEOUT.as_secs()
        )),
    }
}

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

/// Host-Key-Prüfung per TOFU (siehe `known_hosts.rs`): unbekannte Schlüssel
/// werden gemerkt, abweichende Schlüssel brechen die Verbindung ab.
struct AcceptAllHandler {
    host: String,
    port: u16,
    mismatch: Arc<Mutex<Option<(String, String)>>>,
}

impl Handler for AcceptAllHandler {
    type Error = russh::Error;

    async fn check_server_key(&mut self, server_public_key: &PublicKey) -> Result<bool, Self::Error> {
        let fingerprint = server_public_key.fingerprint(HashAlg::Sha256).to_string();
        match crate::known_hosts::check(&self.host, self.port, &fingerprint) {
            crate::known_hosts::Decision::Known | crate::known_hosts::Decision::New => Ok(true),
            crate::known_hosts::Decision::Mismatch { stored } => {
                if let Ok(mut m) = self.mismatch.lock() {
                    *m = Some((stored, fingerprint));
                }
                Ok(false)
            }
        }
    }
}

fn host_key_mismatch_message(host: &str, port: u16, stored: &str, presented: &str) -> String {
    format!(
        "WARNUNG: Der SSH-Schlüssel von {host}:{port} hat sich geändert. Die Verbindung wurde aus Sicherheitsgründen abgebrochen.\n\
         Gespeicherter Fingerprint: {stored}\n\
         Neuer Fingerprint: {presented}\n\
         Das kann ein Man-in-the-Middle-Angriff sein - oder der Server wurde neu installiert. \
         Nur wenn Sie die Änderung erwartet haben: unter Einstellungen › Verbindung beim SSH-Zugang \
         „Schlüssel vergessen“ wählen und neu verbinden."
    )
}

async fn open_session(config: &SshConfig) -> Result<Handle<AcceptAllHandler>, String> {
    let ssh_config = Arc::new(Config::default());
    let mismatch = Arc::new(Mutex::new(None));
    let handler = AcceptAllHandler {
        host: config.host.clone(),
        port: config.port,
        mismatch: mismatch.clone(),
    };
    let result = tokio::time::timeout(
        SSH_TIMEOUT,
        connect(ssh_config, (config.host.as_str(), config.port), handler),
    )
    .await
    .map_err(|_| {
        format!(
            "Verbindung zu {}:{} nach {}s ohne Antwort abgebrochen (Server nicht erreichbar, Firewall, oder Netzwerkproblem).",
            config.host,
            config.port,
            SSH_TIMEOUT.as_secs()
        )
    })?;
    if let Some((stored, presented)) = mismatch.lock().ok().and_then(|m| m.clone()) {
        return Err(host_key_mismatch_message(&config.host, config.port, &stored, &presented));
    }
    result.map_err(|e| e.to_string())
}

// Remembers, per (host, port, username), whether the last successful login
// needed the keyboard-interactive fallback below - every SSH/SFTP call in
// this app opens its own fresh session and re-authenticates from scratch
// (no session reuse), so without this, every single one of them would waste
// an auth attempt on a `password` method already known to be rejected by
// this server. On a server configured with a low `MaxAuthTries` that can
// lock the app out after just 1-2 remote operations even with a correct
// password. Best-effort only: a `false`/missing entry just means "try
// password first, same as before" - it never blocks the keyboard-interactive
// fallback from still running if password unexpectedly fails.
static NEEDS_KEYBOARD_INTERACTIVE: OnceLock<Mutex<HashMap<(String, u16, String), bool>>> = OnceLock::new();

fn auth_cache_key(config: &SshConfig) -> (String, u16, String) {
    (config.host.clone(), config.port, config.username.clone())
}

// Some servers (this FreeBSD/PAM setup among them) only advertise
// "publickey,keyboard-interactive" and reject the plain SSH "password"
// method outright, even though the login is conceptually just a password.
// Fall back to keyboard-interactive, answering every text prompt with the
// given password (matches how ssh/plink behave against the same server).
async fn authenticate_with_password(
    session: &mut Handle<AcceptAllHandler>,
    config: &SshConfig,
    password: &str,
) -> Result<bool, String> {
    let cache = NEEDS_KEYBOARD_INTERACTIVE.get_or_init(|| Mutex::new(HashMap::new()));
    let key = auth_cache_key(config);
    let skip_password = cache
        .lock()
        .ok()
        .and_then(|m| m.get(&key).copied())
        .unwrap_or(false);

    if !skip_password {
        let direct = session
            .authenticate_password(&config.username, password)
            .await
            .map_err(|e| e.to_string())?;
        if direct.success() {
            if let Ok(mut m) = cache.lock() {
                m.insert(key, false);
            }
            return Ok(true);
        }
    }

    let mut response = session
        .authenticate_keyboard_interactive_start(&config.username, None)
        .await
        .map_err(|e| e.to_string())?;

    loop {
        match response {
            KeyboardInteractiveAuthResponse::Success => {
                if let Ok(mut m) = cache.lock() {
                    m.insert(key, true);
                }
                return Ok(true);
            }
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
    config: &SshConfig,
    auth: &SshAuth,
) -> Result<bool, String> {
    tokio::time::timeout(SSH_TIMEOUT, authenticate_inner(session, config, auth))
        .await
        .map_err(|_| {
            format!(
                "Anmeldung bei {}:{} nach {}s ohne Antwort abgebrochen.",
                config.host,
                config.port,
                SSH_TIMEOUT.as_secs()
            )
        })?
}

async fn authenticate_inner(
    session: &mut Handle<AcceptAllHandler>,
    config: &SshConfig,
    auth: &SshAuth,
) -> Result<bool, String> {
    match auth {
        SshAuth::Password { password } => authenticate_with_password(session, config, password).await,
        SshAuth::PrivateKey { path, passphrase } => {
            let key = load_secret_key(path, passphrase.as_deref())
                .map_err(|e| format!("Privater Schlüssel konnte nicht geladen werden: {e}"))?;
            let result = session
                .authenticate_publickey(&config.username, PrivateKeyWithHashAlg::new(Arc::new(key), None))
                .await
                .map_err(|e| e.to_string())?;
            Ok(result.success())
        }
    }
}

pub async fn test_connection(config: &SshConfig, auth: &SshAuth) -> Result<(), String> {
    let mut session = open_session(config).await?;
    if authenticate(&mut session, config, auth).await? {
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
    on_output: F,
) -> Result<CommandResult, String>
where
    F: FnMut(&str),
{
    run_command_with_timeout(config, auth, command, Some(DEFAULT_COMMAND_TIMEOUT), on_output).await
}

/// Wie `run_command_streaming`, aber mit wählbarem Gesamtlimit für die
/// Befehlsausführung (`None` = unbegrenzt, z.B. für sehr lange Builds).
/// Verbindungsaufbau, Anmeldung und Kanalöffnung haben immer ihr eigenes
/// Zeitlimit.
pub async fn run_command_with_timeout<F>(
    config: &SshConfig,
    auth: &SshAuth,
    command: &str,
    limit: Option<Duration>,
    mut on_output: F,
) -> Result<CommandResult, String>
where
    F: FnMut(&str),
{
    let mut session = open_session(config).await?;
    if !authenticate(&mut session, config, auth).await? {
        return Err("Authentifizierung fehlgeschlagen".into());
    }

    let mut channel = tokio::time::timeout(SSH_TIMEOUT, session.channel_open_session())
        .await
        .map_err(|_| "Kanal konnte nicht geöffnet werden (Zeitüberschreitung).".to_string())?
        .map_err(|e| e.to_string())?;
    tokio::time::timeout(SSH_TIMEOUT, channel.exec(true, command))
        .await
        .map_err(|_| "Befehl konnte nicht gestartet werden (Zeitüberschreitung).".to_string())?
        .map_err(|e| e.to_string())?;

    let mut output = String::new();
    let mut exit_status = None;

    let run = async {
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
    };
    match limit {
        Some(d) => {
            if tokio::time::timeout(d, run).await.is_err() {
                return Err(format!(
                    "Befehl nach {} Minuten ohne Abschluss abgebrochen (Zeitüberschreitung).",
                    (d.as_secs() / 60).max(1)
                ));
            }
        }
        None => run.await,
    }

    Ok(CommandResult {
        output,
        exit_status,
    })
}

/// `pub(crate)` (not just module-private) so callers that need to batch
/// several operations over one connection - see
/// `read_file_if_exists_via`/`write_file_with_backup_via`'s own doc - can
/// open a session once and reuse it themselves instead of going through the
/// one-shot `read_remote_file*`/`write_remote_file*` functions below.
pub(crate) async fn open_sftp(config: &SshConfig, auth: &SshAuth) -> Result<SftpSession, String> {
    let mut session = open_session(config).await?;
    if !authenticate(&mut session, config, auth).await? {
        return Err("Authentifizierung fehlgeschlagen".into());
    }
    let channel = sftp_op("SFTP-Kanal konnte nicht geöffnet werden", session.channel_open_session()).await?;
    sftp_op("SFTP-Subsystem konnte nicht gestartet werden", channel.request_subsystem(true, "sftp")).await?;
    sftp_op("SFTP-Sitzung konnte nicht gestartet werden", SftpSession::new(channel.into_stream())).await
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

/// Same as `list_remote_dir`, but a directory that doesn't exist yet is
/// treated as an empty listing instead of an error. For directories this app
/// owns and creates lazily itself (e.g. the DB-backup folder, only ever
/// created by `mkdir -p` the first time a backup actually succeeds - see
/// `db_backup.rs::build_dump_command`), "not found" just means "nothing
/// written here yet", not a misconfiguration - showing a raw SFTP error on
/// the very first visit to that page is just noise. Matches on the SFTP
/// protocol's own `NoSuchFile` status rather than string-sniffing the error
/// message, so it can't accidentally swallow an unrelated failure (wrong
/// permissions, connection drop, etc.) that happens to also mention "file".
pub async fn list_remote_dir_or_empty(
    config: &SshConfig,
    auth: &SshAuth,
    path: &str,
) -> Result<Vec<RemoteEntry>, String> {
    let sftp = open_sftp(config, auth).await?;
    match sftp.read_dir(path).await {
        Ok(entries) => {
            let mut result: Vec<RemoteEntry> = entries
                .map(|entry| RemoteEntry {
                    name: entry.file_name(),
                    is_dir: entry.file_type().is_dir(),
                })
                .collect();
            result.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
            Ok(result)
        }
        Err(russh_sftp::client::error::Error::Status(status))
            if status.status_code == russh_sftp::protocol::StatusCode::NoSuchFile =>
        {
            Ok(Vec::new())
        }
        Err(e) => Err(format!("Ordner konnte nicht gelesen werden ({path}): {e}")),
    }
}

/// Finds the most recent "own" (`.bak`) backup this app created for `path`
/// via `write_remote_file_with_backup`'s `m2manager_backups`-next-to-the-file
/// convention (see `backups.rs`'s module doc for the exact naming scheme) -
/// the shared lookup behind a "letzte Änderung rückgängig machen" button.
/// Reuses `list_remote_dir_or_empty` so a file that was never edited before
/// (no backups folder yet) just means "nothing to undo", not an error.
pub async fn latest_own_backup(
    config: &SshConfig,
    auth: &SshAuth,
    path: &str,
) -> Result<Option<String>, String> {
    let (dir, filename) = match path.rfind('/') {
        Some(idx) => (&path[..idx], &path[idx + 1..]),
        None => (".", path),
    };
    let backup_dir = format!("{dir}/m2manager_backups");
    let entries = list_remote_dir_or_empty(config, auth, &backup_dir).await?;
    Ok(pick_latest_backup(filename, &entries).map(|name| format!("{backup_dir}/{name}")))
}

/// Pure selection logic behind `latest_own_backup`, split out so it's
/// testable without a live SFTP connection.
fn pick_latest_backup(filename: &str, entries: &[RemoteEntry]) -> Option<String> {
    let prefix = format!("{filename}.");
    let mut candidates: Vec<&str> = entries
        .iter()
        .filter(|e| !e.is_dir && e.name.starts_with(&prefix) && e.name.ends_with(".bak"))
        .map(|e| e.name.as_str())
        .collect();
    // The `<timestamp>.bak` suffix is `YYYYMMDD_HHMMSS.bak` (fixed width),
    // so lexicographic order is chronological order.
    candidates.sort();
    candidates.last().map(|s| s.to_string())
}

/// Renames an existing remote file into a sibling `m2manager_backups` folder
/// with a timestamp suffix, without writing anything back - the shared core
/// of `delete_remote_file_with_backup`, `write_remote_file_with_backup`, and
/// `backup_remote_binary`. Returns the backup path, or `None` if `path`
/// doesn't exist (nothing to back up - e.g. a brand-new file).
async fn backup_existing(sftp: &SftpSession, path: &str, suffix: &str) -> Result<Option<String>, String> {
    if !exists(sftp, path).await? {
        return Ok(None);
    }
    let dest = backup_destination(sftp, path, suffix).await;
    sftp_op("Backup fehlgeschlagen", sftp.rename(path, &dest)).await?;
    Ok(Some(dest))
}

/// Legt (falls nötig) den Backup-Ordner neben `path` an und liefert den
/// Zielpfad `<dir>/m2manager_backups/<datei>.<zeitstempel>.<suffix>`.
async fn backup_destination(sftp: &SftpSession, path: &str, suffix: &str) -> String {
    let (dir, filename) = match path.rfind('/') {
        Some(idx) => (&path[..idx], &path[idx + 1..]),
        None => (".", path),
    };
    let backup_dir = format!("{dir}/m2manager_backups");
    if !exists(sftp, &backup_dir).await.unwrap_or(false) {
        let _ = sftp_op("Backup-Ordner", sftp.create_dir(&backup_dir)).await;
    }
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    format!("{backup_dir}/{filename}.{timestamp}.{suffix}")
}

/// Schreibt `bytes` in eine (neue/überschriebene) Datei, schließt sie
/// vollständig und prüft danach die Größe auf dem Server.
async fn write_and_verify(sftp: &SftpSession, path: &str, bytes: &[u8]) -> Result<(), String> {
    let what = format!("Datei konnte nicht geschrieben werden ({path})");
    let mut file = sftp_op(
        &what,
        sftp.open_with_flags(path, OpenFlags::CREATE | OpenFlags::TRUNCATE | OpenFlags::WRITE),
    )
    .await?;
    sftp_op(&what, file.write_all(bytes)).await?;
    sftp_op(&what, file.shutdown()).await?;
    drop(file);
    let size = sftp_op(&what, sftp.metadata(path)).await?.size;
    if let Some(size) = size {
        if size != bytes.len() as u64 {
            return Err(format!(
                "Datei konnte nicht vollständig geschrieben werden ({path}): {size} von {} Bytes angekommen.",
                bytes.len()
            ));
        }
    }
    Ok(())
}

/// Atomares Schreiben: erst nach `<pfad>.m2tmp`, Größe prüfen, dann das
/// Original (`original`, falls vorhanden) als Kopie nach `m2manager_backups`
/// sichern und zuletzt die Temp-Datei auf den Zielpfad umbenennen. Fehler in
/// irgendeiner Phase entfernen die Temp-Datei; das Original bleibt bis zum
/// letzten Schritt unangetastet. Liefert den Backup-Pfad.
async fn write_atomic(
    sftp: &SftpSession,
    path: &str,
    bytes: &[u8],
    original: Option<&[u8]>,
) -> Result<Option<String>, String> {
    let tmp = format!("{path}.m2tmp");
    let result = write_atomic_inner(sftp, path, &tmp, bytes, original).await;
    if result.is_err() {
        let _ = sftp_op("Temp-Datei", sftp.remove_file(tmp.as_str())).await;
    }
    result
}

async fn write_atomic_inner(
    sftp: &SftpSession,
    path: &str,
    tmp: &str,
    bytes: &[u8],
    original: Option<&[u8]>,
) -> Result<Option<String>, String> {
    write_and_verify(sftp, tmp, bytes).await?;

    let backup_path = match original {
        Some(orig) => {
            let dest = backup_destination(sftp, path, "bak").await;
            write_and_verify(sftp, &dest, orig)
                .await
                .map_err(|e| format!("Backup fehlgeschlagen: {e}"))?;
            Some(dest)
        }
        None => None,
    };

    if sftp_op("Umbenennen", sftp.rename(tmp, path)).await.is_err() {
        // Manche SFTP-Server (OpenSSH, SFTP v3) überschreiben beim Umbenennen
        // nicht - dann das Ziel entfernen (Backup liegt bereits) und erneut versuchen.
        sftp_op("Altes Original entfernen", sftp.remove_file(path)).await?;
        if let Err(e) = sftp_op("Umbenennen", sftp.rename(tmp, path)).await {
            if let Some(orig) = original {
                let _ = write_and_verify(sftp, path, orig).await;
            }
            return Err(format!(
                "Datei konnte nicht ersetzt werden ({path}): {e}. Der ursprüngliche Inhalt wurde wiederhergestellt."
            ));
        }
    }
    Ok(backup_path)
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
    backup_existing(&sftp, path, "deleted").await
}

/// Renames the currently-live file at `path` into its sibling
/// `m2manager_backups` folder (same-filesystem rename, no data transfer)
/// without writing a replacement - used as the deploy sequence's backup step
/// before a freshly-built binary is copied into place via a server-side `cp`
/// (see `build_deploy.rs`). Unlike text files, the replacement here never
/// round-trips through this app, so there is no matching
/// `write_remote_binary_with_backup` - this is a standalone step.
pub async fn backup_remote_binary(config: &SshConfig, auth: &SshAuth, path: &str) -> Result<Option<String>, String> {
    let sftp = open_sftp(config, auth).await?;
    backup_existing(&sftp, path, "bak").await
}

/// Permanently removes a remote file - unlike `delete_remote_file_with_backup`,
/// no backup is made first. Used for DB dump files, which are themselves
/// already a backup/snapshot rather than a live server file, so renaming one
/// into another backup folder before deleting it would be pointless churn.
pub async fn delete_remote_file(config: &SshConfig, auth: &SshAuth, path: &str) -> Result<(), String> {
    let sftp = open_sftp(config, auth).await?;
    sftp_op(&format!("Löschen fehlgeschlagen ({path})"), sftp.remove_file(path)).await
}

// Quest/config source files on this server are a mix of plain UTF-8 and
// Windows-1252 (whatever the quest designer's editor happened to save as -
// e.g. `Biologie/Biochecker.lua` has raw cp1252 umlaut bytes, which aren't
// valid UTF-8 and used to make this fail outright). Try strict UTF-8 first
// so already-clean files decode byte-for-byte; only fall back to a cp1252
// decode (which - unlike UTF-8 - never fails, every byte maps to something)
// for files that actually need it. `pub(crate)` so local (non-SSH) file
// reads can reuse the exact same fallback instead of duplicating it - see
// `commands::read_target_content`'s local branch.
pub(crate) fn decode_bytes(bytes: Vec<u8>) -> String {
    match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(e) => {
            let (text, _, _) = encoding_rs::WINDOWS_1252.decode(e.as_bytes());
            text.into_owned()
        }
    }
}

/// Encodes `content` back to bytes matching whatever encoding `original`
/// (the file's own bytes before this write) was in - `decode_bytes` above
/// only ever falls back to Windows-1252 when strict UTF-8 fails, so if
/// `original` isn't valid UTF-8 either, the file must have been Windows-1252
/// on disk and has to be written back the same way. Without this, every
/// write path that reads a file via `decode_bytes` and later saves it back
/// (quest .lua files, client source patches, ...) would silently convert a
/// Windows-1252 file (e.g. containing German umlauts) to UTF-8 on its very
/// first edit - mojibake for every downstream reader still expecting
/// Windows-1252, compounding on every subsequent save of the same file.
/// `original: None` (brand-new file) keeps the existing UTF-8 default.
///
/// Nur eine Datei, die bereits echtes (nicht-ASCII) UTF-8 enthält, bleibt
/// UTF-8. Alles andere - Windows-1252-Dateien, reine ASCII-Dateien und neue
/// Dateien - wird als Windows-1252 geschrieben, dem Standard der
/// Metin2-Serverdateien; eine bisher reine ASCII-Datei wird also nicht
/// plötzlich zu UTF-8. Zeichen, die Windows-1252 nicht darstellen kann,
/// führen zu einem Fehler statt zu stiller Ersetzung.
pub(crate) fn encode_matching(content: &str, original: Option<&[u8]>) -> Result<Vec<u8>, String> {
    let keep_utf8 = matches!(original, Some(bytes) if !bytes.is_ascii() && std::str::from_utf8(bytes).is_ok());
    if keep_utf8 {
        return Ok(content.as_bytes().to_vec());
    }
    let (encoded, _, had_unmappable) = encoding_rs::WINDOWS_1252.encode(content);
    if had_unmappable {
        let bad: Vec<String> = content
            .chars()
            .filter(|c| {
                let mut buf = [0u8; 4];
                encoding_rs::WINDOWS_1252.encode(c.encode_utf8(&mut buf)).2
            })
            .take(5)
            .map(|c| format!("„{c}“"))
            .collect();
        return Err(format!(
            "Der Text enthält Zeichen, die in der Windows-1252-Kodierung der Serverdatei nicht darstellbar sind ({}). Bitte entfernen oder ersetzen.",
            bad.join(", ")
        ));
    }
    Ok(encoded.into_owned())
}

pub async fn read_remote_file(config: &SshConfig, auth: &SshAuth, path: &str) -> Result<String, String> {
    let sftp = open_sftp(config, auth).await?;
    Ok(decode_bytes(read_bytes(&sftp, path).await?))
}

/// Raw byte read, bypassing `decode_bytes` entirely - used where the file's
/// content must never be interpreted as text at all (e.g. restoring a
/// backup byte-for-byte, which might be binary), since even the
/// UTF-8-or-Windows-1252 fallback in `decode_bytes` is lossy for the small
/// set of byte values Windows-1252 leaves undefined (0x81/0x8D/0x8F/0x90/
/// 0x9D decode to U+FFFD and can't be recovered on re-encode).
pub async fn read_remote_file_bytes(config: &SshConfig, auth: &SshAuth, path: &str) -> Result<Vec<u8>, String> {
    let sftp = open_sftp(config, auth).await?;
    read_bytes(&sftp, path).await
}

/// True if `bytes` looks like binary content rather than text - used only to
/// decide whether a read-only diff view should attempt to render decoded
/// text (a NUL byte never appears in genuine text content on this project's
/// supported encodings, but is extremely common in binaries).
pub(crate) fn looks_binary(bytes: &[u8]) -> bool {
    bytes.contains(&0)
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
    read_file_if_exists_via(&sftp, path).await
}

/// Session-reusing counterpart to `read_remote_file_if_exists` - see
/// `open_sftp`'s own doc for why a caller would want this instead.
pub(crate) async fn read_file_if_exists_via(sftp: &SftpSession, path: &str) -> Result<Option<String>, String> {
    if !exists(sftp, path).await? {
        return Ok(None);
    }
    Ok(Some(decode_bytes(read_bytes(sftp, path).await?)))
}

/// Byte-level counterpart to `read_remote_file_if_exists` - see
/// `read_remote_file_bytes`'s own doc for why a caller would want raw bytes.
pub async fn read_remote_file_bytes_if_exists(
    config: &SshConfig,
    auth: &SshAuth,
    path: &str,
) -> Result<Option<Vec<u8>>, String> {
    let sftp = open_sftp(config, auth).await?;
    if !exists(&sftp, path).await? {
        return Ok(None);
    }
    Ok(Some(read_bytes(&sftp, path).await?))
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
        let result = read_bytes(&sftp, path).await.map(decode_bytes);
        results.push(result);
    }
    Ok(results)
}

/// Creates `dir` and every missing ancestor, one segment at a time - SFTP's
/// `create_dir` only creates a single level (unlike `mkdir -p`), so a write
/// into a brand-new subfolder (e.g. a quest category that's never existed
/// before, like "Broadcast") would otherwise fail with "No such file"
/// (real live report: `create_quest_file` writing
/// `share/quest/Broadcast/Broadcast_System.lua` for the first time). Already-
/// existing segments are left untouched; errors on `create_dir` are ignored
/// the same way `backup_existing`'s backup-folder creation already does,
/// since a segment that appeared between the `try_exists` check and the
/// `create_dir` call (or one we simply lack permission to stat) shouldn't
/// abort the write - the follow-up `open_with_flags` below is the real
/// failure signal if the directory genuinely isn't usable.
async fn ensure_remote_dir(sftp: &SftpSession, dir: &str) {
    if dir.is_empty() || dir == "/" || exists(sftp, dir).await.unwrap_or(false) {
        return;
    }
    let mut current = String::new();
    for segment in dir.split('/') {
        if segment.is_empty() {
            continue;
        }
        current.push('/');
        current.push_str(segment);
        if !exists(sftp, &current).await.unwrap_or(false) {
            let _ = sftp_op("Ordner anlegen", sftp.create_dir(&current)).await;
        }
    }
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
    write_file_with_backup_via(&sftp, path, content).await
}

/// Session-reusing counterpart to `write_remote_file_with_backup` - see
/// `open_sftp`'s own doc for why a caller would want this instead.
pub(crate) async fn write_file_with_backup_via(
    sftp: &SftpSession,
    path: &str,
    content: &str,
) -> Result<Option<String>, String> {
    if let Some(idx) = path.rfind('/') {
        ensure_remote_dir(sftp, &path[..idx]).await;
    }

    // Einmal lesen: die Bytes dienen sowohl der Kodierungserkennung (siehe
    // encode_matching) als auch als Inhalt der Backup-Kopie.
    let original_bytes = if exists(sftp, path).await? {
        Some(read_bytes(sftp, path).await?)
    } else {
        None
    };

    let encoded = encode_matching(content, original_bytes.as_deref())?;
    write_atomic(sftp, path, &encoded, original_bytes.as_deref()).await
}

/// Byte-exact write, bypassing `encode_matching`/UTF-8 entirely - same
/// backup-then-overwrite shape as `write_remote_file_with_backup`, but for
/// callers that already have the exact target bytes (e.g. restoring a
/// backup verbatim) and must not risk any text (re-)interpretation at all.
pub async fn write_remote_file_bytes_with_backup(
    config: &SshConfig,
    auth: &SshAuth,
    path: &str,
    content: &[u8],
) -> Result<Option<String>, String> {
    let sftp = open_sftp(config, auth).await?;

    if let Some(idx) = path.rfind('/') {
        ensure_remote_dir(&sftp, &path[..idx]).await;
    }

    let original_bytes = if exists(&sftp, path).await? {
        Some(read_bytes(&sftp, path).await?)
    } else {
        None
    };
    write_atomic(&sftp, path, content, original_bytes.as_deref()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, is_dir: bool) -> RemoteEntry {
        RemoteEntry { name: name.to_string(), is_dir }
    }

    #[test]
    fn pick_latest_backup_returns_the_newest_matching_bak() {
        let entries = vec![
            entry("special_item_group.txt.20260801_090000.bak", false),
            entry("special_item_group.txt.20260825_120000.bak", false),
            entry("special_item_group.txt.20260810_150000.bak", false),
        ];
        assert_eq!(
            pick_latest_backup("special_item_group.txt", &entries),
            Some("special_item_group.txt.20260825_120000.bak".to_string())
        );
    }

    #[test]
    fn pick_latest_backup_ignores_deleted_backups_and_other_files() {
        let entries = vec![
            entry("special_item_group.txt.20260825_120000.deleted", false),
            entry("cube.txt.20260825_130000.bak", false),
            entry("special_item_group.txt.20260825_120000.bak", true), // a directory, not a file
        ];
        assert_eq!(pick_latest_backup("special_item_group.txt", &entries), None);
    }

    #[test]
    fn pick_latest_backup_returns_none_when_no_backups_exist_yet() {
        assert_eq!(pick_latest_backup("special_item_group.txt", &[]), None);
    }

    #[test]
    fn decode_bytes_reads_clean_utf8_unchanged() {
        assert_eq!(decode_bytes("hello".as_bytes().to_vec()), "hello");
    }

    #[test]
    fn decode_bytes_falls_back_to_windows_1252_for_raw_cp1252_bytes() {
        // Reale Live-Meldung eines Nutzers beim System-Installer: eine
        // lokale Client-Quellcode-Datei mit deutschem Umlaut in cp1252
        // (0xE4 = "ä" in Windows-1252, keine gültige UTF-8-Bytefolge) ließ
        // `std::fs::read_to_string` bisher mit "stream did not contain
        // valid UTF-8" hart abbrechen.
        let bytes = vec![b'K', 0xE4, b'f', b'e', b'r']; // "Käfer" in cp1252
        assert_eq!(decode_bytes(bytes), "Käfer");
    }

    #[test]
    fn encode_matching_round_trips_windows_1252_original_bytes() {
        // Reale gemeldete Regression: eine cp1252-Datei mit Umlauten wurde
        // bei jedem Speichern still nach UTF-8 hochkonvertiert, weil das
        // Schreiben nie geprüft hat, wie die Datei ursprünglich kodiert war.
        let original_cp1252 = vec![b'K', 0xE4, b'f', b'e', b'r']; // "Käfer"
        let decoded = decode_bytes(original_cp1252.clone());
        let re_encoded = encode_matching(&decoded, Some(&original_cp1252)).unwrap();
        assert_eq!(re_encoded, original_cp1252);
    }

    #[test]
    fn encode_matching_keeps_real_utf8_files_utf8() {
        let original = "Käfer".as_bytes();
        assert_eq!(encode_matching("Größe", Some(original)).unwrap(), "Größe".as_bytes().to_vec());
        // auch Zeichen außerhalb von cp1252 sind in UTF-8-Dateien erlaubt
        assert_eq!(encode_matching("日本", Some(original)).unwrap(), "日本".as_bytes().to_vec());
    }

    #[test]
    fn encode_matching_does_not_upgrade_ascii_or_new_files_to_utf8() {
        assert_eq!(encode_matching("hello", None).unwrap(), b"hello".to_vec());
        // reine ASCII-Datei + neues "ä" => cp1252 (0xE4), nicht UTF-8 (0xC3 0xA4)
        assert_eq!(encode_matching("ä", Some(b"plain ascii")).unwrap(), vec![0xE4]);
        assert_eq!(encode_matching("ä", None).unwrap(), vec![0xE4]);
    }

    #[test]
    fn encode_matching_rejects_chars_outside_windows_1252() {
        let err = encode_matching("Drache 龍", Some(b"ascii")).unwrap_err();
        assert!(err.contains("龍"), "{err}");
        assert!(encode_matching("x", Some(&[0xE4])).is_ok());
        assert!(encode_matching("日", Some(&[0xE4])).is_err());
    }

    #[test]
    fn host_key_mismatch_message_names_both_fingerprints() {
        let m = host_key_mismatch_message("srv", 22, "SHA256:alt", "SHA256:neu");
        assert!(m.contains("SHA256:alt") && m.contains("SHA256:neu") && m.contains("srv:22"));
        assert!(m.contains("Schlüssel vergessen"));
    }

    #[test]
    fn looks_binary_detects_nul_bytes_but_not_plain_text() {
        assert!(!looks_binary("normal text".as_bytes()));
        assert!(looks_binary(&[0x7F, 0x45, 0x4C, 0x46, 0x00, 0x01]));
    }

    // Regression test for the SSH_TIMEOUT addition (2026-08-11, real live
    // report: System-Installer's "Anwenden" step against a 10-server-file
    // package hung forever with no error - traced to open_session's connect()
    // having no timeout at all, compounded by one fresh SSH connection per
    // file instead of a shared session). Connects against this machine's
    // real configured dev server to prove the new timeout wrapper doesn't
    // slow down or break a genuinely healthy connection - same live-DB-style
    // test pattern already used in db/item.rs and commands.rs.
    #[tokio::test]
    #[ignore = "braucht einen echten Server per SSH (in M2Manager eingerichtete Verbindung) - mit `cargo test -- --ignored` ausführen"]
    async fn healthy_connection_succeeds_well_under_the_timeout() {
        let get = crate::test_support::setting;
        let host = get("ssh_host").expect("ssh_host not configured on this machine");
        let port: u16 = get("ssh_port").unwrap_or_else(|| "22".into()).parse().unwrap();
        let username = get("ssh_username").expect("ssh_username not configured");
        let password =
            crate::credentials::get_secret("ssh_password").expect("ssh_password credential not stored");

        let config = SshConfig { host, port, username };
        let auth = SshAuth::Password { password };

        let start = std::time::Instant::now();
        test_connection(&config, &auth).await.expect("real SSH login should succeed");
        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_secs(10),
            "a healthy connection took {elapsed:?} - well past what a working SSH login should need, even with the new timeout wrapper in place"
        );
    }
}
