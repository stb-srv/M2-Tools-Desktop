//! CSV-Export (Semikolon-getrennt + UTF-8-BOM, damit deutsches Excel die
//! Spalten und Umlaute direkt richtig erkennt).

use crate::tauri::{dialog_save, invoke_unit, DialogFilter, SaveDialogOptions};
use serde_json::json;

fn field(s: &str) -> String {
    if s.contains(';') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

pub fn build_csv(headers: &[String], rows: &[Vec<String>]) -> String {
    let mut out = String::from("\u{feff}");
    for row in std::iter::once(headers).chain(rows.iter().map(|r| r.as_slice())) {
        out.push_str(&row.iter().map(|f| field(f)).collect::<Vec<_>>().join(";"));
        out.push_str("\r\n");
    }
    out
}

/// Fragt per Speichern-Dialog nach dem Ziel und schreibt die CSV. `Ok(false)`
/// = Dialog abgebrochen.
pub async fn export_rows_as_csv(default_file_name: &str, headers: Vec<String>, rows: Vec<Vec<String>>) -> Result<bool, String> {
    let path = dialog_save(&SaveDialogOptions {
        default_path: Some(default_file_name.into()),
        filters: vec![DialogFilter::new("CSV", &["csv"])],
        ..Default::default()
    })
    .await?;
    let Some(path) = path else { return Ok(false) };
    let csv = build_csv(&headers, &rows);
    invoke_unit("export_text_file", &json!({ "path": path, "content": csv })).await?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_fields() {
        let csv = build_csv(&["a".into(), "b;c".into()], &[vec!["x\"y".into(), "1".into()]]);
        assert_eq!(csv, "\u{feff}a;\"b;c\"\r\n\"x\"\"y\";1\r\n");
    }
}
