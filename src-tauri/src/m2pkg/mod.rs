//! Paket-Import (Format `m2pkg` v1, erzeugt vom DB-Tool): ein eigener,
//! vom bisherigen Datei-/Ordner-Importer unabhängiger Weg. Alles (Items,
//! Namen, Dateizuordnung, Server-SQL) kommt aus `manifest.json` und den
//! Dateien des Pakets, nichts wird aus gefundenen Dateien erraten.
//!
//! Ablauf: `Package::open` (Version + Prüfsummen) → `inspect` (Vorschau) →
//! `import` (Backup, Client, Server) → `undo` (anhand des Backups).

mod apply;
pub mod package;
pub mod sql;
pub mod textmerge;

pub use apply::*;
pub use package::Package;
