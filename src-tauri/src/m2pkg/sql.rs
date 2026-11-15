//! SQL-Dateien des Pakets: Statements trennen, nach Vnum filtern.

/// Zerlegt eine SQL-Datei in Statements. Kommentarzeilen (`-- …`) außerhalb
/// von Strings werden verworfen, Semikolons in Strings zählen nicht.
pub fn split_statements(sql: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quote = false;
    let mut escaped = false;
    for line in sql.lines() {
        if !in_quote && cur.trim().is_empty() && line.trim_start().starts_with("--") {
            continue;
        }
        for ch in line.chars() {
            if in_quote {
                cur.push(ch);
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '\'' {
                    in_quote = false;
                }
            } else if ch == '\'' {
                in_quote = true;
                cur.push(ch);
            } else if ch == ';' {
                let stmt = cur.trim();
                if !stmt.is_empty() {
                    out.push(stmt.to_string());
                }
                cur.clear();
            } else {
                cur.push(ch);
            }
        }
        cur.push('\n');
    }
    let rest = cur.trim();
    if !rest.is_empty() {
        out.push(rest.to_string());
    }
    out
}

/// Vnum eines `REPLACE/INSERT … VALUES (<vnum>, …)`-Statements.
pub fn statement_vnum(stmt: &str) -> Option<u32> {
    let pos = stmt.find("VALUES")?;
    let rest = stmt[pos + "VALUES".len()..].trim_start().strip_prefix('(')?;
    let digits: String = rest.trim_start().chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// Statements der gewählten Vnum, Reihenfolge wie in der Datei.
pub fn filter_statements(sql: &str, wanted: &dyn Fn(u32) -> bool) -> Vec<(u32, String)> {
    split_statements(sql)
        .into_iter()
        .filter_map(|s| statement_vnum(&s).filter(|v| wanted(*v)).map(|v| (v, s)))
        .collect()
}

/// Zahl bleibt Zahl, alles andere wird als SQL-String geschrieben.
pub fn sql_literal(value: Option<&str>) -> String {
    match value {
        None => "NULL".to_string(),
        Some(v) if v.parse::<i128>().is_ok() => v.to_string(),
        Some(v) => format!("'{}'", v.replace('\\', "\\\\").replace('\'', "''")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "-- kopf\n-- noch einer\nREPLACE INTO player.item_proto (`vnum`, `name`) VALUES (10, x'41', 'a;b');\nREPLACE INTO player.item_proto (`vnum`, `name`) VALUES (11, x'42', 'it''s');\n";

    #[test]
    fn splits_and_ignores_comments_and_quoted_semicolons() {
        let s = split_statements(SAMPLE);
        assert_eq!(s.len(), 2);
        assert!(s[0].contains("'a;b'"));
        assert!(s[1].contains("'it''s'"));
    }

    #[test]
    fn vnum_and_filter() {
        let f = filter_statements(SAMPLE, &|v| v == 11);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].0, 11);
    }

    #[test]
    fn literals() {
        assert_eq!(sql_literal(Some("42")), "42");
        assert_eq!(sql_literal(Some("-1")), "-1");
        assert_eq!(sql_literal(Some("PARA,STUN")), "'PARA,STUN'");
        assert_eq!(sql_literal(Some("")), "''");
        assert_eq!(sql_literal(None), "NULL");
        assert_eq!(sql_literal(Some("a'b")), "'a''b'");
    }
}
