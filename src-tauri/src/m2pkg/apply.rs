//! Vorschau, Einspielen und Rückgängigmachen eines Pakets.

use super::package::{sha256_file, Assumption, ManifestFile, Package, Warning};
use super::sql;
use super::textmerge::{self, ReplacedLine};
use base64::Engine;
use serde::{Deserialize, Serialize};
use sqlx::{MySqlPool, Row};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

const CLIENT_PREFIX: &str = "client/";
const BACKUP_SUBDIR: &str = "m2manager_backups/m2pkg";

/// Alle 44 Spalten von `player.item_proto` (Reihenfolge wie im Paket-SQL).
const PROTO_COLUMNS: &[&str] = &[
    "vnum", "vnum_range", "name", "locale_name", "type", "subtype", "weight", "size", "antiflag", "flag",
    "wearflag", "immuneflag", "gold", "shop_buy_price", "refined_vnum", "refine_set", "magic_pct", "limittype0",
    "limitvalue0", "limittype1", "limitvalue1", "applytype0", "applyvalue0", "applytype1", "applyvalue1",
    "applytype2", "applyvalue2", "applytype3", "applyvalue3", "value0", "value1", "value2", "value3", "value4",
    "value5", "socket0", "socket1", "socket2", "socket3", "socket4", "socket5", "specular", "socket_pct",
    "addon_type",
];
/// `VARBINARY`-Spalten werden im Backup als Hex gesichert.
const BINARY_COLUMNS: &[&str] = &["name", "locale_name"];

// ------------------------------------------------------------------ Vorschau

#[derive(Debug, Clone, Serialize)]
pub struct InspectFile {
    pub path: String,
    pub role: String,
    pub guessed: bool,
    pub size: u64,
    /// Wie viele Items des Pakets diese Datei benutzen.
    pub used_by: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct InspectItem {
    pub vnum: u32,
    pub name: Option<String>,
    pub item_type: String,
    pub icon_path: Option<String>,
    pub model_path: Option<String>,
    pub icon_data_url: Option<String>,
    pub files: Vec<InspectFile>,
    pub assumed_antiflags: Vec<String>,
    pub warnings: Vec<Warning>,
    /// Gibt es für dieses Item eine Zeile im Server-SQL?
    pub has_proto: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Inspect {
    pub zip_path: String,
    pub format_version: u64,
    pub dbtool_version: Option<String>,
    pub created: Option<String>,
    pub collection: String,
    pub client_version: Option<String>,
    pub proto_source: Option<String>,
    pub assumptions: Vec<Assumption>,
    pub global_warnings: Vec<Warning>,
    pub items: Vec<InspectItem>,
    pub file_count: usize,
    pub shared_file_count: usize,
    pub has_sql: bool,
}

fn icon_data_url(bytes: &[u8], path: &str) -> Option<String> {
    let format = if path.to_ascii_lowercase().ends_with(".tga") {
        image::ImageFormat::Tga
    } else {
        image::guess_format(bytes).ok()?
    };
    let img = image::load_from_memory_with_format(bytes, format).ok()?;
    let mut png = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).ok()?;
    Some(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png)))
}

fn sql_vnums(pkg: &Package) -> Result<BTreeSet<u32>, String> {
    let Some(bytes) = pkg.read_optional("server/item_proto_replace.sql")? else {
        return Ok(BTreeSet::new());
    };
    Ok(sql::filter_statements(&String::from_utf8_lossy(&bytes), &|_| true).into_iter().map(|(v, _)| v).collect())
}

pub fn inspect(pkg: &Package, zip_path: &str) -> Result<Inspect, String> {
    let m = &pkg.manifest;
    let proto = sql_vnums(pkg)?;
    let mut usage: HashMap<&str, usize> = HashMap::new();
    for item in &m.items {
        for f in &item.files {
            *usage.entry(f.path.as_str()).or_default() += 1;
        }
    }
    let mut icons: HashMap<String, Option<String>> = HashMap::new();
    let mut items = Vec::new();
    for item in &m.items {
        let icon_file = item
            .files
            .iter()
            .find(|f| f.role == "icon" && !f.guessed)
            .or_else(|| item.files.iter().find(|f| f.role == "icon"));
        let icon = match icon_file {
            Some(f) => icons
                .entry(f.path.clone())
                .or_insert_with(|| pkg.read(&f.path).ok().and_then(|b| icon_data_url(&b, &f.path)))
                .clone(),
            None => None,
        };
        items.push(InspectItem {
            vnum: item.vnum,
            name: item.name.clone(),
            item_type: item.item_type.clone(),
            icon_path: item.icon_path.clone(),
            model_path: item.model_path.clone(),
            icon_data_url: icon,
            files: item
                .files
                .iter()
                .map(|f| InspectFile {
                    path: f.path.clone(),
                    role: f.role.clone(),
                    guessed: f.guessed,
                    size: f.size,
                    used_by: usage.get(f.path.as_str()).copied().unwrap_or(1),
                })
                .collect(),
            assumed_antiflags: item.assumed_antiflags.iter().map(|a| a.name.clone()).collect(),
            warnings: m.warnings.iter().filter(|w| w.vnum == Some(item.vnum)).cloned().collect(),
            has_proto: proto.contains(&item.vnum),
        });
    }
    Ok(Inspect {
        zip_path: zip_path.to_string(),
        format_version: m.format_version,
        dbtool_version: m.dbtool_version.clone(),
        created: m.created.clone(),
        collection: m.collection_name(),
        client_version: m.client_version.clone(),
        proto_source: m.proto_source.clone(),
        assumptions: m.assumptions.clone(),
        global_warnings: m.warnings.iter().filter(|w| w.vnum.is_none()).cloned().collect(),
        items,
        file_count: usage.len(),
        shared_file_count: usage.values().filter(|n| **n > 1).count(),
        has_sql: !proto.is_empty(),
    })
}

// ------------------------------------------------------------------ Optionen

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConflictMode {
    Overwrite,
    Keep,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApplyOptions {
    pub zip_path: String,
    pub vnums: Vec<u32>,
    /// Vermutete Dateien (`guessed`), die ausdrücklich mitgenommen werden sollen.
    #[serde(default)]
    pub guessed_paths: Vec<String>,
    #[serde(default)]
    pub confirm_assumptions: bool,
    /// Eigener Pfad zur `item_list.txt` des Clients (sonst automatische Suche).
    #[serde(default)]
    pub item_list_path: Option<String>,
    /// Eigener Pfad zur `item_names.txt` des Clients (sonst automatische Suche).
    #[serde(default)]
    pub item_names_path: Option<String>,
    /// SQL direkt in MySQL schreiben.
    pub write_db: bool,
    /// SQL-/Server-Dateien in einen Ordner schreiben.
    pub write_files: bool,
    #[serde(default)]
    pub out_dir: Option<String>,
    pub conflict: ConflictMode,
}

fn selected_vnums(pkg: &Package, opts: &ApplyOptions) -> Result<HashSet<u32>, String> {
    if opts.vnums.is_empty() {
        return Err("Es ist kein Item ausgewählt.".into());
    }
    let known: HashSet<u32> = pkg.manifest.items.iter().map(|i| i.vnum).collect();
    if let Some(v) = opts.vnums.iter().find(|v| !known.contains(v)) {
        return Err(format!("Vnum {v} steht nicht im Paket."));
    }
    Ok(opts.vnums.iter().copied().collect())
}

/// Dateien (ZIP-Pfad → Eintrag), die für die gewählten Items kopiert werden:
/// alles Sichere, vermutete Dateien nur wenn ausdrücklich angehakt.
fn selected_files<'a>(pkg: &'a Package, vnums: &HashSet<u32>, guessed_ok: &HashSet<&str>) -> BTreeMap<&'a str, &'a ManifestFile> {
    let mut out = BTreeMap::new();
    for item in pkg.manifest.items.iter().filter(|i| vnums.contains(&i.vnum)) {
        for f in &item.files {
            if !f.guessed || guessed_ok.contains(f.path.as_str()) {
                out.insert(f.path.as_str(), f);
            }
        }
    }
    out
}

fn client_target(client_dir: &Path, zip_path: &str) -> Result<PathBuf, String> {
    let rel = zip_path.strip_prefix(CLIENT_PREFIX).ok_or_else(|| format!("Kein Client-Pfad: {zip_path}"))?;
    Ok(rel.split('/').fold(client_dir.to_path_buf(), |p, c| p.join(c)))
}

/// Findet eine Textdatei des Clients automatisch: in jedem `locale/<Sprache>/`,
/// direkt in `locale/` und im Client-Ordner selbst.
fn locale_files(client_dir: &Path, file_name: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let locale = client_dir.join("locale");
    if let Ok(rd) = std::fs::read_dir(&locale) {
        for e in rd.flatten() {
            let p = e.path().join(file_name);
            if p.is_file() {
                found.push(p);
            }
        }
    }
    found.sort();
    for p in [locale.join(file_name), client_dir.join(file_name)] {
        if p.is_file() {
            found.push(p);
        }
    }
    found
}

/// Ziel(e) einer Textdatei: ein von Hand angegebener Pfad hat Vorrang und
/// ersetzt die automatische Suche; ohne ihn wird gesucht. Leer = nicht gefunden.
fn text_targets(client_dir: &Path, file_name: &str, custom: Option<&str>) -> Result<Vec<PathBuf>, String> {
    match custom.map(str::trim).filter(|c| !c.is_empty()) {
        Some(c) => {
            let p = PathBuf::from(c);
            if p.is_file() {
                Ok(vec![p])
            } else {
                Err(format!("Der angegebene Pfad für {file_name} existiert nicht: {c}"))
            }
        }
        None => Ok(locale_files(client_dir, file_name)),
    }
}

// ------------------------------------------------------------------ Vorprüfung

#[derive(Debug, Clone, Serialize)]
pub struct Preflight {
    /// `None`, wenn keine MySQL-Verbindung besteht.
    pub existing_in_db: Option<Vec<u32>>,
    pub existing_in_client_list: Vec<u32>,
    /// Dateien im Client, die schon da sind, aber anderen Inhalt haben.
    pub differing_files: Vec<String>,
    pub identical_files: usize,
    pub item_list_files: Vec<String>,
    pub item_names_files: Vec<String>,
    pub notes: Vec<String>,
}

pub async fn preflight(
    pkg: Package,
    client_dir: PathBuf,
    opts: ApplyOptions,
    pool: Option<MySqlPool>,
) -> Result<Preflight, String> {
    let (pkg, client_dir, opts) = (&pkg, client_dir.as_path(), &opts);
    let vnums = selected_vnums(pkg, opts)?;
    let guessed: HashSet<&str> = opts.guessed_paths.iter().map(|s| s.as_str()).collect();
    let mut differing = Vec::new();
    let mut identical = 0;
    for (zip_path, f) in selected_files(pkg, &vnums, &guessed) {
        let target = client_target(client_dir, zip_path)?;
        if target.is_file() {
            if sha256_file(&target)? == f.sha256.to_ascii_lowercase() {
                identical += 1;
            } else {
                differing.push(zip_path.trim_start_matches(CLIENT_PREFIX).to_string());
            }
        }
    }

    let list_files = text_targets(client_dir, "item_list.txt", opts.item_list_path.as_deref())?;
    let names_files = text_targets(client_dir, "item_names.txt", opts.item_names_path.as_deref())?;
    let mut in_list = BTreeSet::new();
    for f in &list_files {
        for l in std::fs::read(f).map_err(|e| e.to_string())?.split(|b| *b == b'\n') {
            if let Some(v) = textmerge::line_vnum(l.strip_suffix(b"\r").unwrap_or(l)).filter(|v| vnums.contains(v)) {
                in_list.insert(v);
            }
        }
    }

    let mut notes = Vec::new();
    if list_files.is_empty() {
        notes.push("Keine item_list.txt gefunden. Ohne sie bekommen die Items im Client kein Icon und kein Modell. Bitte unten den Pfad angeben, sonst wird die Datei übersprungen.".into());
    }
    if names_files.is_empty() {
        notes.push("Keine item_names.txt gefunden. Ohne Pfad werden die Namen dort nicht eingetragen (übersprungen).".into());
    }
    let existing_in_db = match pool {
        Some(pool) => Some(existing_db_vnums(&pool, &vnums.iter().copied().collect::<Vec<_>>()).await?.into_iter().collect()),
        None => None,
    };
    Ok(Preflight {
        existing_in_db,
        existing_in_client_list: in_list.into_iter().collect(),
        differing_files: differing,
        identical_files: identical,
        item_list_files: list_files.iter().map(|p| p.display().to_string()).collect(),
        item_names_files: names_files.iter().map(|p| p.display().to_string()).collect(),
        notes,
    })
}

// ------------------------------------------------------------------ Journal

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextEdit {
    /// Pfad relativ zum Client-Ordner (mit `/`).
    pub rel: String,
    /// Absoluter Pfad, wenn die Datei außerhalb des Client-Ordners liegt (eigener Pfad).
    #[serde(default)]
    pub abs: Option<String>,
    pub added: Vec<u32>,
    pub replaced: Vec<ReplacedLine>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Journal {
    pub id: String,
    pub created_at: String,
    pub collection: String,
    pub package_file: String,
    pub vnums: Vec<u32>,
    pub files_created: Vec<String>,
    pub files_replaced: Vec<String>,
    pub text_edits: Vec<TextEdit>,
    pub db_applied: bool,
    /// Vnum, die es in `item_proto` vorher nicht gab (beim Rückgängigmachen löschen).
    pub db_new_vnums: Vec<u32>,
    /// Vnum, deren Zeile überschrieben wurde (aus `item_proto_before.sql` wiederherstellen).
    pub db_replaced_vnums: Vec<u32>,
    pub undone: bool,
}

fn journal_path(backup_dir: &Path) -> PathBuf {
    backup_dir.join("journal.json")
}

fn save_journal(backup_dir: &Path, j: &Journal) -> Result<(), String> {
    let text = serde_json::to_string_pretty(j).map_err(|e| e.to_string())?;
    std::fs::write(journal_path(backup_dir), text).map_err(|e| format!("Journal nicht schreibbar: {e}"))
}

fn load_journal(backup_dir: &Path) -> Result<Journal, String> {
    let text = std::fs::read_to_string(journal_path(backup_dir)).map_err(|e| format!("Journal nicht lesbar: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("Journal fehlerhaft: {e}"))
}

fn backups_root(client_dir: &Path) -> PathBuf {
    BACKUP_SUBDIR.split('/').fold(client_dir.to_path_buf(), |p, c| p.join(c))
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_digit() || c == '_')
}

// ------------------------------------------------------------------ Einspielen

#[derive(Debug, Clone, Default, Serialize)]
pub struct ApplyReport {
    pub backup_id: String,
    pub backup_dir: String,
    pub items_selected: usize,
    pub files_created: usize,
    pub files_replaced: usize,
    pub files_unchanged: usize,
    pub files_kept: Vec<String>,
    pub list_added: usize,
    pub list_replaced: usize,
    pub list_kept: usize,
    pub db_written: Option<DbReport>,
    pub server_files: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DbReport {
    pub inserted: usize,
    pub replaced: usize,
    pub kept: usize,
}

fn stamp_dir(root: &Path) -> Result<(String, PathBuf), String> {
    std::fs::create_dir_all(root).map_err(|e| format!("Backup-Ordner nicht anlegbar: {e}"))?;
    let base = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    for n in 0..100 {
        let id = if n == 0 { base.clone() } else { format!("{base}_{n}") };
        let dir = root.join(&id);
        if std::fs::create_dir(&dir).is_ok() {
            return Ok((id, dir));
        }
    }
    Err("Backup-Ordner nicht anlegbar.".into())
}

fn needs_assumption_confirmation(pkg: &Package, vnums: &HashSet<u32>) -> Vec<String> {
    let mut names = BTreeSet::new();
    for i in pkg.manifest.items.iter().filter(|i| vnums.contains(&i.vnum)) {
        for a in &i.assumed_antiflags {
            names.insert(a.name.clone());
        }
    }
    names.into_iter().collect()
}

/// Zustand eines Einspiel-Laufs. Alles Synchrone steckt in den Methoden; die
/// `async`-Funktion `import` hält über ihre `await`-Stellen nur eigene Werte.
struct Run {
    pkg: Package,
    client_dir: PathBuf,
    opts: ApplyOptions,
    vnums: HashSet<u32>,
    statements: Vec<(u32, String)>,
    sql_name: &'static str,
    sql_text: String,
    report: ApplyReport,
    journal: Journal,
    backup_dir: PathBuf,
    existing_db: HashSet<u32>,
}

impl Run {
    /// Prüft alles und legt Backup-Ordner und Journal an, ohne etwas zu ändern.
    fn prepare(pkg: Package, client_dir: PathBuf, opts: ApplyOptions, has_pool: bool) -> Result<Run, String> {
        let vnums = selected_vnums(&pkg, &opts)?;
        if !client_dir.is_dir() {
            return Err(format!("Client-Ordner „{}“ existiert nicht.", client_dir.display()));
        }
        if !opts.write_db && !opts.write_files {
            return Err("Bei „Server“ muss mindestens eine Möglichkeit gewählt sein (direkt in MySQL und/oder SQL-Dateien).".into());
        }
        let flags = needs_assumption_confirmation(&pkg, &vnums);
        if !flags.is_empty() && !opts.confirm_assumptions {
            return Err(format!(
                "Die AntiFlags {} beruhen auf einer Annahme (Standard-Layout von Metin2). Bitte vor dem Einspielen ausdrücklich bestätigen.",
                flags.join(", ")
            ));
        }
        // eigene Pfade vor dem ersten Schreiben prüfen
        text_targets(&client_dir, "item_list.txt", opts.item_list_path.as_deref())?;
        text_targets(&client_dir, "item_names.txt", opts.item_names_path.as_deref())?;
        if opts.write_db && !has_pool {
            return Err("Keine aktive MySQL-Verbindung. Bitte zuerst verbinden oder „nur SQL-Dateien“ wählen.".into());
        }
        let sql_name: &'static str = if opts.conflict == ConflictMode::Overwrite {
            "server/item_proto_replace.sql"
        } else {
            "server/item_proto_insert_ignore.sql"
        };
        let sql_text = String::from_utf8(
            pkg.read_optional(sql_name)?.ok_or_else(|| format!("{sql_name} fehlt im Paket."))?,
        )
        .map_err(|_| format!("{sql_name} ist kein gültiger Text."))?;
        let statements = sql::filter_statements(&sql_text, &|v| vnums.contains(&v));

        let mut report = ApplyReport { items_selected: vnums.len(), ..Default::default() };
        let have: HashSet<u32> = statements.iter().map(|(v, _)| *v).collect();
        let mut without_proto: Vec<u32> = vnums.iter().copied().filter(|v| !have.contains(v)).collect();
        without_proto.sort_unstable();
        if !without_proto.is_empty() {
            report.notes.push(format!(
                "Für {} Item(s) enthält das Paket keine item_proto-Zeile ({}). Sie bekommen nur die Client-Einträge.",
                without_proto.len(),
                join_vnums(&without_proto)
            ));
        }

        let (id, backup_dir) = stamp_dir(&backups_root(&client_dir))?;
        report.backup_id = id.clone();
        report.backup_dir = backup_dir.display().to_string();
        let mut sorted: Vec<u32> = vnums.iter().copied().collect();
        sorted.sort_unstable();
        let journal = Journal {
            id,
            created_at: chrono::Local::now().to_rfc3339(),
            collection: pkg.manifest.collection_name(),
            package_file: Path::new(&opts.zip_path).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default(),
            vnums: sorted,
            ..Default::default()
        };
        save_journal(&backup_dir, &journal)?;
        Ok(Run {
            pkg, client_dir, opts, vnums, statements, sql_name, sql_text, report, journal, backup_dir,
            existing_db: HashSet::new(),
        })
    }

    fn proto_vnums(&self) -> Vec<u32> {
        self.statements.iter().map(|(v, _)| *v).collect()
    }

    /// Client-Dateien und Textdateien; bei einem Fehler wird alles zurückgenommen.
    fn client_step(&mut self) -> Result<(), String> {
        let res = apply_client(
            &self.pkg, &self.client_dir, &self.backup_dir, &self.opts, &self.vnums, &mut self.journal, &mut self.report,
        );
        let _ = save_journal(&self.backup_dir, &self.journal);
        match res {
            Ok(()) => Ok(()),
            Err(e) => Err(match undo_client(&self.client_dir, &self.backup_dir, &self.journal) {
                Ok(()) => format!("{e}\nBereits Geschriebenes wurde zurückgenommen."),
                Err(e2) => format!(
                    "{e}\nAchtung, das Zurücknehmen war nicht vollständig: {e2}\nBackup: {}",
                    self.backup_dir.display()
                ),
            }),
        }
    }

    fn after_db(&mut self, result: Result<DbReport, String>) -> Result<(), String> {
        match result {
            Ok(db) => {
                self.journal.db_applied = true;
                for (v, _) in &self.statements {
                    if self.existing_db.contains(v) {
                        if self.opts.conflict == ConflictMode::Overwrite {
                            self.journal.db_replaced_vnums.push(*v);
                        }
                    } else {
                        self.journal.db_new_vnums.push(*v);
                    }
                }
                self.report.db_written = Some(db);
                let _ = save_journal(&self.backup_dir, &self.journal);
                Ok(())
            }
            Err(e) => {
                let _ = undo_client(&self.client_dir, &self.backup_dir, &self.journal);
                Err(format!(
                    "Schreiben in MySQL fehlgeschlagen, in der Datenbank wurde nichts geändert (Client-Änderungen sind zurückgenommen): {e}"
                ))
            }
        }
    }

    fn finish(mut self) -> ApplyReport {
        if self.opts.write_files {
            match write_server_files(&self.pkg, &self.opts, &self.vnums, self.sql_name, &self.sql_text) {
                Ok(paths) => self.report.server_files = paths,
                Err(e) => self.report.notes.push(format!("Server-Dateien konnten nicht geschrieben werden: {e}")),
            }
        }
        crate::icons::invalidate_caches();
        self.report
    }
}

/// Sichert die vorhandenen `item_proto`-Zeilen und liefert deren Vnum.
async fn db_backup(pool: MySqlPool, proto_vnums: Vec<u32>, backup_dir: PathBuf) -> Result<HashSet<u32>, String> {
    let existing: HashSet<u32> = existing_db_vnums(&pool, &proto_vnums).await?.into_iter().collect();
    if !existing.is_empty() {
        let mut sorted: Vec<u32> = existing.iter().copied().collect();
        sorted.sort_unstable();
        let backup_sql = backup_proto_rows(&pool, &sorted).await?;
        std::fs::write(backup_dir.join("item_proto_before.sql"), backup_sql)
            .map_err(|e| format!("Backup der item_proto-Zeilen nicht schreibbar: {e}"))?;
    }
    Ok(existing)
}

async fn db_write(
    pool: MySqlPool,
    statements: Vec<(u32, String)>,
    existing: HashSet<u32>,
    conflict: ConflictMode,
) -> Result<DbReport, String> {
    apply_sql(&pool, &statements, &existing, conflict).await
}

/// Einspielen: Prüfen → Backup (Dateien, Textzeilen, item_proto) → Client →
/// MySQL → Server-Dateien. Scheitert ein Schritt, wird der Client zurückgenommen.
pub async fn import(
    pkg: Package,
    client_dir: PathBuf,
    opts: ApplyOptions,
    pool: Option<MySqlPool>,
) -> Result<ApplyReport, String> {
    let mut run = Run::prepare(pkg, client_dir, opts, pool.is_some())?;
    let pool = if run.opts.write_db { pool } else { None };
    if let Some(p) = pool.clone() {
        run.existing_db = db_backup(p, run.proto_vnums(), run.backup_dir.clone()).await?;
    }
    run.client_step()?;
    if let Some(p) = pool {
        let result = db_write(p, run.statements.clone(), run.existing_db.clone(), run.opts.conflict).await;
        run.after_db(result)?;
    }
    Ok(run.finish())
}

fn join_vnums(v: &[u32]) -> String {
    v.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(", ")
}

fn rel_of(client_dir: &Path, p: &Path) -> String {
    p.strip_prefix(client_dir)
        .unwrap_or(p)
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn backup_copy(client_dir: &Path, backup_dir: &Path, target: &Path) -> Result<String, String> {
    let inside = target.starts_with(client_dir);
    let rel = if inside {
        rel_of(client_dir, target)
    } else {
        // eigener Pfad außerhalb des Clients: Sicherung unter _extern/
        let n = std::fs::read_dir(backup_dir.join("files").join("_extern")).map(|r| r.count()).unwrap_or(0);
        format!("_extern/{n}_{}", target.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default())
    };
    let dest = rel.split('/').fold(backup_dir.join("files"), |p, c| p.join(c));
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Backup nicht anlegbar: {e}"))?;
    }
    std::fs::copy(target, &dest).map_err(|e| format!("Backup von {} fehlgeschlagen: {e}", target.display()))?;
    Ok(rel)
}

fn apply_client(
    pkg: &Package,
    client_dir: &Path,
    backup_dir: &Path,
    opts: &ApplyOptions,
    vnums: &HashSet<u32>,
    journal: &mut Journal,
    report: &mut ApplyReport,
) -> Result<(), String> {
    let overwrite = opts.conflict == ConflictMode::Overwrite;
    let guessed: HashSet<&str> = opts.guessed_paths.iter().map(|s| s.as_str()).collect();

    // Dateien: geteilte Dateien stehen einmal im ZIP und werden einmal kopiert.
    for (zip_path, f) in selected_files(pkg, vnums, &guessed) {
        let target = client_target(client_dir, zip_path)?;
        let rel = rel_of(client_dir, &target);
        if target.exists() {
            if !target.is_file() {
                return Err(format!("{rel} ist im Client ein Ordner, keine Datei."));
            }
            if sha256_file(&target)? == f.sha256.to_ascii_lowercase() {
                report.files_unchanged += 1;
                continue;
            }
            if !overwrite {
                report.files_kept.push(rel);
                continue;
            }
            backup_copy(client_dir, backup_dir, &target)?;
            journal.files_replaced.push(rel.clone());
            let _ = save_journal(backup_dir, journal);
            write_verified(pkg, zip_path, &target, &f.sha256)?;
            report.files_replaced += 1;
        } else {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
            }
            journal.files_created.push(rel);
            let _ = save_journal(backup_dir, journal);
            write_verified(pkg, zip_path, &target, &f.sha256)?;
            report.files_created += 1;
        }
    }

    // item_list.txt (Pflicht) und item_names.txt (wenn vorhanden)
    let list_bytes = pkg.read_optional("client/item_list.txt")?;
    let names_bytes = pkg.read_optional("client/item_names.txt")?;
    let wanted = |v: u32| vnums.contains(&v);
    for (bytes, file_name, is_list) in [(list_bytes, "item_list.txt", true), (names_bytes, "item_names.txt", false)] {
        let Some(bytes) = bytes else { continue };
        let lines = textmerge::package_lines(&bytes, &wanted);
        let custom = if is_list { opts.item_list_path.as_deref() } else { opts.item_names_path.as_deref() };
        let targets = text_targets(client_dir, file_name, custom)?;
        if targets.is_empty() {
            report.notes.push(format!(
                "ÜBERSPRUNGEN: {file_name} wurde im Client nicht gefunden, es wurde nichts eingetragen. Pfad im Reiter angeben und erneut einspielen."
            ));
            continue;
        }
        for target in targets {
            let existing = std::fs::read(&target).map_err(|e| format!("{}: {e}", target.display()))?;
            let merged = textmerge::merge_lines(&existing, &lines, overwrite);
            if merged.bytes == existing {
                if is_list {
                    report.list_kept += merged.kept.len();
                }
                continue;
            }
            let rel = backup_copy(client_dir, backup_dir, &target)?;
            let abs = (!target.starts_with(client_dir)).then(|| target.display().to_string());
            journal.text_edits.push(TextEdit { rel: rel.clone(), abs, added: merged.added.clone(), replaced: merged.replaced.clone() });
            let _ = save_journal(backup_dir, journal);
            std::fs::write(&target, &merged.bytes).map_err(|e| format!("{}: {e}", target.display()))?;
            if is_list {
                report.list_added += merged.added.len();
                report.list_replaced += merged.replaced.len();
                report.list_kept += merged.kept.len();
            }
        }
    }
    Ok(())
}

/// Schreibt über eine Temp-Datei und prüft die Prüfsumme, bevor sie ans Ziel kommt.
fn write_verified(pkg: &Package, zip_path: &str, target: &Path, expected_sha: &str) -> Result<(), String> {
    let tmp = target.with_extension(format!(
        "{}.m2pkg-tmp",
        target.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default()
    ));
    let actual = pkg.extract_to(zip_path, &tmp)?;
    if actual != expected_sha.to_ascii_lowercase() {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("Prüfsumme von {zip_path} stimmt beim Entpacken nicht."));
    }
    std::fs::rename(&tmp, target).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("{}: {e}", target.display())
    })
}

// ------------------------------------------------------------------ MySQL

async fn existing_db_vnums(pool: &MySqlPool, vnums: &[u32]) -> Result<Vec<u32>, String> {
    let mut found = Vec::new();
    for chunk in vnums.chunks(500) {
        let marks = vec!["?"; chunk.len()].join(",");
        let q = format!("SELECT CAST(vnum AS CHAR) AS v FROM player.item_proto WHERE vnum IN ({marks})");
        let mut query = sqlx::query(&q);
        for v in chunk {
            query = query.bind(*v);
        }
        for row in query.fetch_all(pool).await.map_err(|e| e.to_string())? {
            let v: String = row.try_get("v").map_err(|e| e.to_string())?;
            if let Ok(n) = v.parse::<u32>() {
                found.push(n);
            }
        }
    }
    Ok(found)
}

/// Sichert bestehende Zeilen als `REPLACE`-SQL, die man auch von Hand einspielen kann.
async fn backup_proto_rows(pool: &MySqlPool, vnums: &[u32]) -> Result<String, String> {
    let select_cols: Vec<String> = PROTO_COLUMNS
        .iter()
        .map(|c| {
            if BINARY_COLUMNS.contains(c) {
                format!("HEX(`{c}`) AS `{c}`")
            } else {
                format!("CAST(`{c}` AS CHAR) AS `{c}`")
            }
        })
        .collect();
    let col_list = PROTO_COLUMNS.iter().map(|c| format!("`{c}`")).collect::<Vec<_>>().join(", ");
    let mut out = String::from("-- M2 Tools Desktop: Sicherung der item_proto-Zeilen vor dem Paket-Import\n");
    for chunk in vnums.chunks(200) {
        let marks = vec!["?"; chunk.len()].join(",");
        let q = format!("SELECT {} FROM player.item_proto WHERE vnum IN ({marks})", select_cols.join(", "));
        let mut query = sqlx::query(&q);
        for v in chunk {
            query = query.bind(*v);
        }
        for row in query.fetch_all(pool).await.map_err(|e| format!("item_proto konnte nicht gesichert werden: {e}"))? {
            let mut values = Vec::new();
            for c in PROTO_COLUMNS {
                let v: Option<String> = row.try_get(*c).map_err(|e| e.to_string())?;
                values.push(if BINARY_COLUMNS.contains(c) {
                    match v {
                        Some(h) => format!("x'{h}'"),
                        None => "NULL".into(),
                    }
                } else {
                    sql::sql_literal(v.as_deref())
                });
            }
            out.push_str(&format!("REPLACE INTO player.item_proto ({col_list}) VALUES ({});\n", values.join(", ")));
        }
    }
    Ok(out)
}

async fn apply_sql(
    pool: &MySqlPool,
    statements: &[(u32, String)],
    existing: &HashSet<u32>,
    conflict: ConflictMode,
) -> Result<DbReport, String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    for (vnum, stmt) in statements {
        sqlx::query(stmt)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Vnum {vnum}: {}", scrub_error(&e.to_string())))?;
    }
    tx.commit().await.map_err(|e| e.to_string())?;
    let mut rep = DbReport::default();
    for (v, _) in statements {
        match (existing.contains(v), conflict) {
            (false, _) => rep.inserted += 1,
            (true, ConflictMode::Overwrite) => rep.replaced += 1,
            (true, ConflictMode::Keep) => rep.kept += 1,
        }
    }
    Ok(rep)
}

/// Fehlertexte können Verbindungsdaten enthalten; die kommen nie in Meldungen.
fn scrub_error(msg: &str) -> String {
    msg.lines().next().unwrap_or("").chars().take(300).collect()
}

// ------------------------------------------------------------------ Server-Dateien

fn safe_name(s: &str) -> String {
    let n: String = s
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    if n.is_empty() { "paket".into() } else { n }
}

fn write_server_files(
    pkg: &Package,
    opts: &ApplyOptions,
    vnums: &HashSet<u32>,
    sql_name: &str,
    sql_text: &str,
) -> Result<Vec<String>, String> {
    let base = match opts.out_dir.as_deref().filter(|s| !s.trim().is_empty()) {
        Some(d) => PathBuf::from(d),
        None => Path::new(&opts.zip_path).parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from(".")),
    };
    let dir = base.join(format!(
        "{}_server_{}",
        safe_name(&pkg.manifest.collection_name()),
        chrono::Local::now().format("%Y%m%d_%H%M%S")
    ));
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut written = Vec::new();

    let stmts = sql::filter_statements(sql_text, &|v| vnums.contains(&v));
    let mut sql_out = format!("-- m2pkg: {} Item(s), {}\n", stmts.len(), sql_name.trim_start_matches("server/"));
    for (_, s) in &stmts {
        sql_out.push_str(s);
        sql_out.push_str(";\n");
    }
    let sql_file = dir.join(sql_name.trim_start_matches("server/"));
    std::fs::write(&sql_file, sql_out).map_err(|e| e.to_string())?;
    written.push(sql_file.display().to_string());

    // item_proto.txt (Serverformat, LF): Kopfzeile + gewählte Zeilen
    if let Some(bytes) = pkg.read_optional("server/item_proto.txt")? {
        let mut out = Vec::new();
        for (i, line) in bytes.split(|b| *b == b'\n').enumerate() {
            let keep = i == 0 || textmerge::line_vnum(line).map(|v| vnums.contains(&v)).unwrap_or(false);
            if keep && !line.is_empty() {
                out.extend_from_slice(line);
                out.push(b'\n');
            }
        }
        let f = dir.join("item_proto.txt");
        std::fs::write(&f, out).map_err(|e| e.to_string())?;
        written.push(f.display().to_string());
    }
    // item_names.txt (Server): Kopfzeile + gewählte Zeilen, CRLF wie im Paket
    if let Some(bytes) = pkg.read_optional("server/item_names.txt")? {
        let mut out = Vec::new();
        for (i, line) in bytes.split(|b| *b == b'\n').enumerate() {
            let line = line.strip_suffix(b"\r").unwrap_or(line);
            let keep = i == 0 || textmerge::line_vnum(line).map(|v| vnums.contains(&v)).unwrap_or(false);
            if keep && !line.is_empty() {
                out.extend_from_slice(line);
                out.extend_from_slice(b"\r\n");
            }
        }
        let f = dir.join("item_names.txt");
        std::fs::write(&f, out).map_err(|e| e.to_string())?;
        written.push(f.display().to_string());
    }
    Ok(written)
}

// ------------------------------------------------------------------ Rückgängig

#[derive(Debug, Clone, Serialize)]
pub struct ImportRecord {
    pub id: String,
    pub created_at: String,
    pub collection: String,
    pub package_file: String,
    pub item_count: usize,
    pub db_applied: bool,
    pub undone: bool,
}

pub fn list_imports(client_dir: &Path) -> Vec<ImportRecord> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(backups_root(client_dir)) {
        for e in rd.flatten() {
            if let Ok(j) = load_journal(&e.path()) {
                out.push(ImportRecord {
                    id: j.id,
                    created_at: j.created_at,
                    collection: j.collection,
                    package_file: j.package_file,
                    item_count: j.vnums.len(),
                    db_applied: j.db_applied,
                    undone: j.undone,
                });
            }
        }
    }
    out.sort_by(|a, b| b.id.cmp(&a.id));
    out
}

/// Nimmt die Client-Änderungen anhand von Journal und Backup zurück.
fn undo_client(client_dir: &Path, backup_dir: &Path, j: &Journal) -> Result<(), String> {
    let mut problems = Vec::new();
    let safe = |rel: &str| super::package::is_safe_zip_path(rel);
    for rel in &j.files_created {
        if !safe(rel) {
            continue;
        }
        let target = rel.split('/').fold(client_dir.to_path_buf(), |p, c| p.join(c));
        if target.is_file() {
            if let Err(e) = std::fs::remove_file(&target) {
                problems.push(format!("{rel}: {e}"));
                continue;
            }
        }
        // leere Ordner, die der Import angelegt hat, wieder entfernen
        let mut dir = target.parent().map(|p| p.to_path_buf());
        while let Some(d) = dir {
            if d == client_dir || !d.starts_with(client_dir) || std::fs::remove_dir(&d).is_err() {
                break;
            }
            dir = d.parent().map(|p| p.to_path_buf());
        }
    }
    for rel in &j.files_replaced {
        if !safe(rel) {
            continue;
        }
        let backup = rel.split('/').fold(backup_dir.join("files"), |p, c| p.join(c));
        let target = rel.split('/').fold(client_dir.to_path_buf(), |p, c| p.join(c));
        if let Err(e) = std::fs::copy(&backup, &target) {
            problems.push(format!("{rel}: {e}"));
        }
    }
    for edit in &j.text_edits {
        let target = match &edit.abs {
            Some(abs) => PathBuf::from(abs),
            None if safe(&edit.rel) => edit.rel.split('/').fold(client_dir.to_path_buf(), |p, c| p.join(c)),
            None => continue,
        };
        match std::fs::read(&target) {
            Ok(cur) => {
                let back = textmerge::unmerge_lines(&cur, &edit.added, &edit.replaced);
                if let Err(e) = std::fs::write(&target, back) {
                    problems.push(format!("{}: {e}", edit.rel));
                }
            }
            Err(e) => problems.push(format!("{}: {e}", edit.rel)),
        }
    }
    crate::icons::invalidate_caches();
    if problems.is_empty() { Ok(()) } else { Err(problems.join("; ")) }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct UndoReport {
    pub files_removed: usize,
    pub files_restored: usize,
    pub text_files_edited: usize,
    pub db_deleted: usize,
    pub db_restored: usize,
}

pub async fn undo(client_dir: PathBuf, id: String, pool: Option<MySqlPool>) -> Result<UndoReport, String> {
    let (client_dir, id) = (client_dir.as_path(), id.as_str());
    if !valid_id(id) {
        return Err("Ungültige Import-Kennung.".into());
    }
    let backup_dir = backups_root(client_dir).join(id);
    let mut j = load_journal(&backup_dir)?;
    if j.undone {
        return Err("Dieser Import wurde schon rückgängig gemacht.".into());
    }
    let mut rep = UndoReport {
        files_removed: j.files_created.len(),
        files_restored: j.files_replaced.len(),
        text_files_edited: j.text_edits.len(),
        ..Default::default()
    };

    // zuerst die Datenbank: ist sie nicht erreichbar, bleibt noch alles wie es ist
    if j.db_applied {
        let pool = pool.as_ref().ok_or("Für das Rückgängigmachen wird die MySQL-Verbindung gebraucht. Bitte zuerst verbinden.")?;
        let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
        for chunk in j.db_new_vnums.chunks(200) {
            let list = chunk.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",");
            let q = format!("DELETE FROM player.item_proto WHERE vnum IN ({list})");
            sqlx::query(&q).execute(&mut *tx).await.map_err(|e| scrub_error(&e.to_string()))?;
        }
        rep.db_deleted = j.db_new_vnums.len();
        if !j.db_replaced_vnums.is_empty() {
            let before = std::fs::read_to_string(backup_dir.join("item_proto_before.sql"))
                .map_err(|e| format!("item_proto_before.sql nicht lesbar: {e}"))?;
            let restore: HashSet<u32> = j.db_replaced_vnums.iter().copied().collect();
            for (vnum, stmt) in sql::filter_statements(&before, &|v| restore.contains(&v)) {
                sqlx::query(&stmt)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| format!("Vnum {vnum}: {}", scrub_error(&e.to_string())))?;
                rep.db_restored += 1;
            }
        }
        tx.commit().await.map_err(|e| e.to_string())?;
    }

    undo_client(client_dir, &backup_dir, &j)?;
    j.undone = true;
    save_journal(&backup_dir, &j)?;
    Ok(rep)
}

#[cfg(test)]
mod tests {
    use super::super::package::test_pkg::*;
    use super::*;

    fn client(tag: &str) -> PathBuf {
        let c = temp_dir(tag).join("client");
        std::fs::create_dir_all(c.join("locale").join("de")).unwrap();
        std::fs::write(c.join("locale/de/item_list.txt"), b"100\tETC\ticon/item/00100.tga\r\n").unwrap();
        std::fs::write(c.join("locale/de/item_names.txt"), b"VNUM\tLOCALE_NAME\r\n100\tAlt\r\n").unwrap();
        c
    }

    fn opts(zip: &Path, vnums: Vec<u32>, conflict: ConflictMode) -> ApplyOptions {
        ApplyOptions {
            zip_path: zip.display().to_string(),
            vnums,
            guessed_paths: vec![],
            confirm_assumptions: true,
            item_list_path: None,
            item_names_path: None,
            write_db: false,
            write_files: true,
            out_dir: None,
            conflict,
        }
    }

    fn run<F: std::future::Future>(f: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread().build().unwrap().block_on(f)
    }

    #[test]
    fn inspect_lists_all_items_with_shared_files() {
        let dir = temp_dir("insp");
        let zip = build(&dir, 1, false, false);
        let pkg = Package::open(&zip).unwrap();
        let i = inspect(&pkg, &zip.display().to_string()).unwrap();
        assert_eq!(i.items.len(), 2);
        assert_eq!(i.items[0].name.as_deref(), Some("Item 300"));
        assert!(i.items.iter().all(|x| x.has_proto));
        assert_eq!(i.file_count, 3);
        assert_eq!(i.shared_file_count, 3);
        assert!(i.items[0].files.iter().any(|f| f.guessed));
        assert_eq!(i.items[0].assumed_antiflags, vec!["ANTI_GIVE"]);
    }

    #[test]
    fn import_requires_assumption_confirmation() {
        let dir = temp_dir("conf");
        let zip = build(&dir, 1, false, false);
        let pkg = Package::open(&zip).unwrap();
        let c = client("conf");
        let mut o = opts(&zip, vec![300], ConflictMode::Overwrite);
        o.confirm_assumptions = false;
        let err = run(import(pkg, c.clone(), o, None)).unwrap_err();
        assert!(err.contains("ANTI_GIVE"), "{err}");
        assert!(!c.join("m2manager_backups").exists(), "vor der Bestätigung darf nichts angelegt werden");
    }

    #[test]
    fn import_then_undo_restores_client_exactly() {
        let dir = temp_dir("imp");
        let zip = build(&dir, 1, false, false);
        let pkg = Package::open(&zip).unwrap();
        let c = client("imp");
        let list_before = std::fs::read(c.join("locale/de/item_list.txt")).unwrap();
        let names_before = std::fs::read(c.join("locale/de/item_names.txt")).unwrap();

        let mut o = opts(&zip, vec![300, 301], ConflictMode::Overwrite);
        o.out_dir = Some(dir.display().to_string());
        let rep = run(import(pkg, c.clone(), o, None)).unwrap();
        // geteilte Dateien nur einmal; die vermutete Textur ist nicht dabei
        assert_eq!(rep.files_created, 2);
        assert!(c.join("icon/item/00300.tga").is_file());
        assert!(c.join("d_/ymir work/item/weapon/00300.gr2").is_file());
        assert!(!c.join("d_/ymir work/item/weapon/00300.dds").exists());
        assert_eq!(rep.list_added, 2);
        let list = std::fs::read(c.join("locale/de/item_list.txt")).unwrap();
        assert!(list.starts_with(&list_before));
        assert!(String::from_utf8_lossy(&list).contains("301\tWEAPON"));
        let names = std::fs::read(c.join("locale/de/item_names.txt")).unwrap();
        assert!(names.windows(7).any(|w| w == b"D\xfcrenda"), "cp1252-Umlaut muss als ein Byte erhalten bleiben");
        assert_eq!(rep.server_files.len(), 3);
        let sql = std::fs::read_to_string(&rep.server_files[0]).unwrap();
        assert!(sql.contains("REPLACE INTO") && sql.contains("(300,") && sql.contains("(301,"));

        let recs = list_imports(&c);
        assert_eq!(recs.len(), 1);
        run(undo(c.clone(), rep.backup_id.clone(), None)).unwrap();
        assert_eq!(std::fs::read(c.join("locale/de/item_list.txt")).unwrap(), list_before);
        assert_eq!(std::fs::read(c.join("locale/de/item_names.txt")).unwrap(), names_before);
        assert!(!c.join("icon").exists(), "leere, vom Import angelegte Ordner werden entfernt");
        assert!(!c.join("d_").exists());
        assert!(list_imports(&c)[0].undone);
        assert!(run(undo(c.clone(), rep.backup_id.clone(), None)).is_err());
    }

    #[test]
    fn only_selected_items_and_guessed_files_on_request() {
        let dir = temp_dir("sel");
        let zip = build(&dir, 1, false, false);
        let pkg = Package::open(&zip).unwrap();
        let c = client("sel");
        let mut o = opts(&zip, vec![301], ConflictMode::Overwrite);
        o.guessed_paths = vec!["client/d_/ymir work/item/weapon/00300.dds".into()];
        o.out_dir = Some(dir.display().to_string());
        let rep = run(import(pkg, c.clone(), o, None)).unwrap();
        assert_eq!(rep.files_created, 3);
        let list = String::from_utf8_lossy(&std::fs::read(c.join("locale/de/item_list.txt")).unwrap()).into_owned();
        assert!(list.contains("301\t") && !list.contains("300\t"));
        let sql = std::fs::read_to_string(&rep.server_files[0]).unwrap();
        assert!(sql.contains("(301,") && !sql.contains("(300,"));
    }

    #[test]
    fn keep_mode_leaves_existing_files_and_lines() {
        let dir = temp_dir("keep");
        let zip = build(&dir, 1, false, false);
        let pkg = Package::open(&zip).unwrap();
        let c = client("keep");
        std::fs::create_dir_all(c.join("icon/item")).unwrap();
        std::fs::write(c.join("icon/item/00300.tga"), b"MEIN-ICON").unwrap();
        std::fs::write(
            c.join("locale/de/item_list.txt"),
            b"300\tETC\ticon/item/mein.tga\r\n",
        )
        .unwrap();
        let mut o = opts(&zip, vec![300], ConflictMode::Keep);
        o.out_dir = Some(dir.display().to_string());
        let rep = run(import(pkg, c.clone(), o, None)).unwrap();
        assert_eq!(rep.files_kept, vec!["icon/item/00300.tga"]);
        assert_eq!(std::fs::read(c.join("icon/item/00300.tga")).unwrap(), b"MEIN-ICON");
        assert_eq!(rep.list_kept, 1);
        let list = std::fs::read(c.join("locale/de/item_list.txt")).unwrap();
        assert_eq!(list, b"300\tETC\ticon/item/mein.tga\r\n");
        let sql = std::fs::read_to_string(&rep.server_files[0]).unwrap();
        assert!(sql.contains("INSERT IGNORE INTO"));
    }

    #[test]
    fn overwrite_mode_backs_up_and_undo_restores_replaced_file() {
        let dir = temp_dir("ovr");
        let zip = build(&dir, 1, false, false);
        let pkg = Package::open(&zip).unwrap();
        let c = client("ovr");
        std::fs::create_dir_all(c.join("icon/item")).unwrap();
        std::fs::write(c.join("icon/item/00300.tga"), b"MEIN-ICON").unwrap();
        let mut o = opts(&zip, vec![300], ConflictMode::Overwrite);
        o.out_dir = Some(dir.display().to_string());
        let rep = run(import(pkg, c.clone(), o, None)).unwrap();
        assert_eq!(rep.files_replaced, 1);
        assert_eq!(std::fs::read(c.join("icon/item/00300.tga")).unwrap(), b"ICONDATA");
        run(undo(c.clone(), rep.backup_id.clone(), None)).unwrap();
        assert_eq!(std::fs::read(c.join("icon/item/00300.tga")).unwrap(), b"MEIN-ICON");
    }

    #[test]
    fn missing_text_files_are_skipped_and_reported_not_fatal() {
        let dir = temp_dir("nolist");
        let zip = build(&dir, 1, false, false);
        let pkg = Package::open(&zip).unwrap();
        let c = temp_dir("nolist2");
        let mut o = opts(&zip, vec![300], ConflictMode::Overwrite);
        o.out_dir = Some(dir.display().to_string());
        let rep = run(import(pkg, c.clone(), o, None)).unwrap();
        assert_eq!(rep.files_created, 2);
        assert_eq!(rep.notes.iter().filter(|n| n.contains("ÜBERSPRUNGEN")).count(), 2, "{:?}", rep.notes);
    }

    #[test]
    fn finds_text_files_without_locale_in_client_root() {
        let dir = temp_dir("root");
        let zip = build(&dir, 1, false, false);
        let pkg = Package::open(&zip).unwrap();
        let c = temp_dir("root2");
        std::fs::write(c.join("item_list.txt"), b"100\tETC\ta.tga\r\n").unwrap();
        let mut o = opts(&zip, vec![300], ConflictMode::Overwrite);
        o.out_dir = Some(dir.display().to_string());
        let rep = run(import(pkg, c.clone(), o, None)).unwrap();
        assert_eq!(rep.list_added, 1);
        assert!(String::from_utf8_lossy(&std::fs::read(c.join("item_list.txt")).unwrap()).contains("300\tWEAPON"));
    }

    #[test]
    fn custom_paths_outside_client_work_and_undo() {
        let dir = temp_dir("custom");
        let zip = build(&dir, 1, false, false);
        let pkg = Package::open(&zip).unwrap();
        let c = temp_dir("custom2");
        let extern_dir = temp_dir("custom3");
        let list = extern_dir.join("meine_liste.txt");
        let names = extern_dir.join("namen.txt");
        std::fs::write(&list, b"100\tETC\ta.tga\r\n").unwrap();
        std::fs::write(&names, b"VNUM\tLOCALE_NAME\r\n").unwrap();
        let mut o = opts(&zip, vec![300], ConflictMode::Overwrite);
        o.item_list_path = Some(list.display().to_string());
        o.item_names_path = Some(names.display().to_string());
        o.out_dir = Some(dir.display().to_string());
        let rep = run(import(pkg, c.clone(), o, None)).unwrap();
        assert_eq!(rep.list_added, 1);
        assert!(std::fs::read(&names).unwrap().len() > 20);
        run(undo(c.clone(), rep.backup_id.clone(), None)).unwrap();
        assert_eq!(std::fs::read(&list).unwrap(), b"100\tETC\ta.tga\r\n");
        assert_eq!(std::fs::read(&names).unwrap(), b"VNUM\tLOCALE_NAME\r\n");
    }

    #[test]
    fn wrong_custom_path_is_an_error_before_writing() {
        let dir = temp_dir("badpath");
        let zip = build(&dir, 1, false, false);
        let pkg = Package::open(&zip).unwrap();
        let c = client("badpath");
        let mut o = opts(&zip, vec![300], ConflictMode::Overwrite);
        o.item_list_path = Some("/gibt/es/nicht.txt".into());
        let err = run(import(pkg, c.clone(), o, None)).unwrap_err();
        assert!(err.contains("existiert nicht"), "{err}");
        assert!(!c.join("icon").exists());
    }

    #[test]
    fn db_mode_without_connection_is_refused() {
        let dir = temp_dir("nodb");
        let zip = build(&dir, 1, false, false);
        let pkg = Package::open(&zip).unwrap();
        let c = client("nodb");
        let mut o = opts(&zip, vec![300], ConflictMode::Overwrite);
        o.write_db = true;
        assert!(run(import(pkg, c.clone(), o, None)).unwrap_err().contains("MySQL"));
        assert!(!c.join("m2manager_backups").exists());
    }

    #[test]
    fn undo_rejects_path_tricks() {
        let c = client("trick");
        assert!(run(undo(c.clone(), "../x".to_string(), None)).is_err());
    }

    /// Mit einem echten Paket: `M2PKG_SAMPLE=/pfad/paket.zip cargo test sample -- --ignored --nocapture`
    #[test]
    #[ignore = "braucht ein echtes Paket (M2PKG_SAMPLE)"]
    fn sample_package_inspects_all_items() {
        let zip = std::env::var("M2PKG_SAMPLE").expect("M2PKG_SAMPLE");
        let pkg = Package::open(Path::new(&zip)).unwrap();
        let i = inspect(&pkg, &zip).unwrap();
        println!("{} Items, {} Dateien, {} geteilt", i.items.len(), i.file_count, i.shared_file_count);
        assert_eq!(i.items.len(), 56);
        assert!(i.items.iter().all(|x| x.has_proto && x.icon_data_url.is_some()));
    }
}


