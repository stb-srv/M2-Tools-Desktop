// Automatisierte Datenbank-Backups: runs `mysqldump`/`mysql` on the remote
// server over the existing SSH connection, reusing the same MySQL
// host/port/username/password already configured for this app's own DB
// connection (see settings.rs / credentials.rs) - the assumption that those
// credentials are also valid *from the server itself* wasn't verified
// against a live setup, but is true whenever the DB and game server are
// colocated (the common case for this kind of hosting), and the user can
// still just use "localhost" as the MySQL host if so.

/// Escapes a string for safe embedding inside a single-quoted POSIX shell
/// argument (`'...'`) - standard technique: close the quote, emit an escaped
/// literal quote, reopen the quote. Used instead of trusting the password
/// (or any other value) to contain no shell metacharacters.
pub fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// The password is passed via the `MYSQL_PWD` environment variable rather
/// than `mysqldump -p...` - the latter is briefly visible to other users on
/// the same machine via `ps`, which `MYSQL_PWD` avoids.
pub fn build_dump_command(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    databases: &[String],
    backup_dir: &str,
    filename: &str,
) -> String {
    let db_args = databases
        .iter()
        .map(|d| shell_single_quote(d))
        .collect::<Vec<_>>()
        .join(" ");
    let full_path = format!("{}/{}", backup_dir.trim_end_matches('/'), filename);
    format!(
        "mkdir -p {dir} && MYSQL_PWD={pw} mysqldump -h {host} -P {port} -u {user} --databases {dbs} > {path}",
        dir = shell_single_quote(backup_dir),
        pw = shell_single_quote(password),
        host = shell_single_quote(host),
        user = shell_single_quote(username),
        dbs = db_args,
        path = shell_single_quote(&full_path),
    )
}

pub fn build_restore_command(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    backup_path: &str,
) -> String {
    format!(
        "MYSQL_PWD={pw} mysql -h {host} -P {port} -u {user} < {path}",
        pw = shell_single_quote(password),
        host = shell_single_quote(host),
        user = shell_single_quote(username),
        path = shell_single_quote(backup_path),
    )
}

pub fn backup_filename(now: chrono::DateTime<chrono::Local>) -> String {
    format!("m2manager_backup_{}.sql", now.format("%Y%m%d_%H%M%S"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn escapes_embedded_single_quotes() {
        assert_eq!(shell_single_quote("it's"), r"'it'\''s'");
        assert_eq!(shell_single_quote("plain"), "'plain'");
    }

    #[test]
    fn builds_dump_command_with_escaped_password() {
        let cmd = build_dump_command(
            "localhost",
            3306,
            "root",
            "p'w",
            &["account".to_string(), "player".to_string()],
            "/home/game/backups",
            "backup_20260101.sql",
        );
        assert_eq!(
            cmd,
            "mkdir -p '/home/game/backups' && MYSQL_PWD='p'\\''w' mysqldump -h 'localhost' -P 3306 -u 'root' --databases 'account' 'player' > '/home/game/backups/backup_20260101.sql'"
        );
    }

    #[test]
    fn builds_restore_command() {
        let cmd = build_restore_command("localhost", 3306, "root", "pw", "/home/game/backups/x.sql");
        assert_eq!(
            cmd,
            "MYSQL_PWD='pw' mysql -h 'localhost' -P 3306 -u 'root' < '/home/game/backups/x.sql'"
        );
    }

    #[test]
    fn backup_filename_uses_timestamp() {
        let dt = chrono::Local.with_ymd_and_hms(2026, 1, 2, 3, 4, 5).unwrap();
        assert_eq!(backup_filename(dt), "m2manager_backup_20260102_030405.sql");
    }
}
