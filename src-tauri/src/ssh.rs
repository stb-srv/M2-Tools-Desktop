use russh::client::{connect, Config, Handle, Handler, KeyboardInteractiveAuthResponse};
use russh::keys::{load_secret_key, PrivateKeyWithHashAlg, PublicKey};
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::OpenFlags;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
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
    mut on_output: F,
) -> Result<CommandResult, String>
where
    F: FnMut(&str),
{
    let mut session = open_session(config).await?;
    if !authenticate(&mut session, config, auth).await? {
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
    if !authenticate(&mut session, config, auth).await? {
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

/// Sucht rekursiv nach Dateien mit genau diesem Namen unter `root` - für den
/// System-Installer, der aus einem Systempaket-Dateinamen (z.B. `cmd_gm.cpp`)
/// erst noch den echten Pfad im Live-Quellbaum finden muss (die
/// Präfix-Konventionen der Pakete stimmen nicht 1:1 mit dem echten Layout
/// überein, z.B. `Source/Server/game/cmd_gm.cpp` im Paket vs.
/// `game/src/cmd_gm.cpp` live). Nutzt `find` statt einer rekursiven
/// SFTP-Traversierung, da das auf dem Server selbst läuft und für tiefe
/// Bäume wie den kompletten Quellcode deutlich schneller ist als viele
/// einzelne SFTP-Roundtrips. Gibt vollständige Pfade zurück, mehrere
/// Treffer möglich (der Aufrufer entscheidet, wie damit umzugehen ist).
pub async fn find_remote_file_by_name(
    config: &SshConfig,
    auth: &SshAuth,
    root: &str,
    filename: &str,
) -> Result<Vec<String>, String> {
    let command = format!(
        "find {} -type f -name {} 2>/dev/null",
        crate::db_backup::shell_single_quote(root),
        crate::db_backup::shell_single_quote(filename)
    );
    let result = run_command_streaming(config, auth, &command, |_| {}).await?;
    Ok(result
        .output
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect())
}

/// Wie `find_remote_file_by_name`, aber sucht viele Dateinamen in einem
/// einzigen `find`-Aufruf statt einen SSH-Roundtrip pro Datei zu bezahlen -
/// bei einem Systempaket mit einem Dutzend Server-Dateien sonst ein Dutzend
/// separater SSH-Verbindungen/-Befehle nacheinander.
pub async fn find_remote_files_by_names(
    config: &SshConfig,
    auth: &SshAuth,
    root: &str,
    filenames: &[String],
) -> Result<std::collections::HashMap<String, Vec<String>>, String> {
    if filenames.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    let name_clauses: Vec<String> = filenames
        .iter()
        .map(|f| format!("-name {}", crate::db_backup::shell_single_quote(f)))
        .collect();
    let command = format!(
        "find {} -type f \\( {} \\) 2>/dev/null",
        crate::db_backup::shell_single_quote(root),
        name_clauses.join(" -o ")
    );
    let result = run_command_streaming(config, auth, &command, |_| {}).await?;

    let mut by_name: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for line in result.output.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if let Some(name) = line.rsplit('/').next() {
            by_name.entry(name.to_string()).or_default().push(line.to_string());
        }
    }
    Ok(by_name)
}

/// Renames an existing remote file into a sibling `m2manager_backups` folder
/// with a timestamp suffix, without writing anything back - the shared core
/// of `delete_remote_file_with_backup`, `write_remote_file_with_backup`, and
/// `backup_remote_binary`. Returns the backup path, or `None` if `path`
/// doesn't exist (nothing to back up - e.g. a brand-new file).
async fn backup_existing(sftp: &SftpSession, path: &str, suffix: &str) -> Result<Option<String>, String> {
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
    let dest = format!("{backup_dir}/{filename}.{timestamp}.{suffix}");
    sftp.rename(path, &dest)
        .await
        .map_err(|e| format!("Backup fehlgeschlagen: {e}"))?;
    Ok(Some(dest))
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
    sftp.remove_file(path).await.map_err(|e| format!("Löschen fehlgeschlagen ({path}): {e}"))
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
pub(crate) fn encode_matching(content: &str, original: Option<&[u8]>) -> Vec<u8> {
    match original {
        Some(bytes) if std::str::from_utf8(bytes).is_err() => {
            let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(content);
            encoded.into_owned()
        }
        _ => content.as_bytes().to_vec(),
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

/// Raw byte read, bypassing `decode_bytes` entirely - used where the file's
/// content must never be interpreted as text at all (e.g. restoring a
/// backup byte-for-byte, which might be binary), since even the
/// UTF-8-or-Windows-1252 fallback in `decode_bytes` is lossy for the small
/// set of byte values Windows-1252 leaves undefined (0x81/0x8D/0x8F/0x90/
/// 0x9D decode to U+FFFD and can't be recovered on re-encode).
pub async fn read_remote_file_bytes(config: &SshConfig, auth: &SshAuth, path: &str) -> Result<Vec<u8>, String> {
    let sftp = open_sftp(config, auth).await?;
    sftp.read(path)
        .await
        .map_err(|e| format!("Datei konnte nicht gelesen werden ({path}): {e}"))
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
    if !sftp.try_exists(path).await.map_err(|e| e.to_string())? {
        return Ok(None);
    }
    let bytes = sftp
        .read(path)
        .await
        .map_err(|e| format!("Datei konnte nicht gelesen werden ({path}): {e}"))?;
    Ok(Some(decode_bytes(bytes)))
}

/// Byte-level counterpart to `read_remote_file_if_exists` - see
/// `read_remote_file_bytes`'s own doc for why a caller would want raw bytes.
pub async fn read_remote_file_bytes_if_exists(
    config: &SshConfig,
    auth: &SshAuth,
    path: &str,
) -> Result<Option<Vec<u8>>, String> {
    let sftp = open_sftp(config, auth).await?;
    if !sftp.try_exists(path).await.map_err(|e| e.to_string())? {
        return Ok(None);
    }
    let bytes = sftp
        .read(path)
        .await
        .map_err(|e| format!("Datei konnte nicht gelesen werden ({path}): {e}"))?;
    Ok(Some(bytes))
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
    if dir.is_empty() || dir == "/" || sftp.try_exists(dir).await.unwrap_or(false) {
        return;
    }
    let mut current = String::new();
    for segment in dir.split('/') {
        if segment.is_empty() {
            continue;
        }
        current.push('/');
        current.push_str(segment);
        if !sftp.try_exists(&current).await.unwrap_or(false) {
            let _ = sftp.create_dir(&current).await;
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

    if let Some(idx) = path.rfind('/') {
        ensure_remote_dir(&sftp, &path[..idx]).await;
    }

    // Peeked *before* backup_existing renames the file away, so this still
    // sees the file's own bytes - see encode_matching's doc for why this
    // matters (preserves Windows-1252 across re-saves instead of silently
    // upgrading to UTF-8).
    let original_bytes = sftp.read(path).await.ok();

    let backup_path = backup_existing(&sftp, path, "bak").await?;

    // sftp.write() only opens with WRITE (no CREATE), which would fail here
    // since the path above was just renamed away - open with CREATE
    // explicitly so this also works for a brand-new file.
    let mut file = sftp
        .open_with_flags(path, OpenFlags::CREATE | OpenFlags::TRUNCATE | OpenFlags::WRITE)
        .await
        .map_err(|e| format!("Datei konnte nicht geschrieben werden ({path}): {e}"))?;
    file.write_all(&encode_matching(content, original_bytes.as_deref()))
        .await
        .map_err(|e| e.to_string())?;
    file.shutdown().await.map_err(|e| e.to_string())?;

    Ok(backup_path)
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

    let backup_path = backup_existing(&sftp, path, "bak").await?;

    let mut file = sftp
        .open_with_flags(path, OpenFlags::CREATE | OpenFlags::TRUNCATE | OpenFlags::WRITE)
        .await
        .map_err(|e| format!("Datei konnte nicht geschrieben werden ({path}): {e}"))?;
    file.write_all(content).await.map_err(|e| e.to_string())?;
    file.shutdown().await.map_err(|e| e.to_string())?;

    Ok(backup_path)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let re_encoded = encode_matching(&decoded, Some(&original_cp1252));
        assert_eq!(re_encoded, original_cp1252);
    }

    #[test]
    fn encode_matching_uses_utf8_for_brand_new_or_already_utf8_files() {
        assert_eq!(encode_matching("hello", None), b"hello".to_vec());
        assert_eq!(
            encode_matching("hello", Some("existing utf-8".as_bytes())),
            b"hello".to_vec()
        );
    }

    #[test]
    fn looks_binary_detects_nul_bytes_but_not_plain_text() {
        assert!(!looks_binary("normal text".as_bytes()));
        assert!(looks_binary(&[0x7F, 0x45, 0x4C, 0x46, 0x00, 0x01]));
    }
}
