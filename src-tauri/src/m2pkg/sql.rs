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

/// Ein Wert aus dem VALUES-Tupel. Nur Literale sind erlaubt.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Int(i64),
    UInt(u64),
    /// Dezimalzahl, als Text gebunden (MySQL wandelt sie in die Spaltenart).
    Dec(String),
    Str(String),
    Bytes(Vec<u8>),
}

/// Ein geprüftes `REPLACE/INSERT [IGNORE] INTO player.item_proto`-Statement.
#[derive(Debug, Clone, PartialEq)]
pub struct ProtoStatement {
    pub replace: bool,
    pub ignore: bool,
    pub columns: Option<Vec<String>>,
    pub values: Vec<Value>,
}

impl ProtoStatement {
    /// Vnum: Wert der Spalte `vnum` (ohne Spaltenliste: der erste Wert).
    pub fn vnum(&self) -> Result<u32, String> {
        let idx = match &self.columns {
            None => 0,
            Some(c) => c.iter().position(|n| n.eq_ignore_ascii_case("vnum")).ok_or("Spalte vnum fehlt")?,
        };
        match self.values.get(idx) {
            Some(Value::Int(n)) => u32::try_from(*n).map_err(|_| "vnum außerhalb des gültigen Bereichs".to_string()),
            Some(Value::UInt(n)) => u32::try_from(*n).map_err(|_| "vnum außerhalb des gültigen Bereichs".to_string()),
            _ => Err("vnum ist keine Ganzzahl".to_string()),
        }
    }

    /// Statement mit `?`-Platzhaltern; die Werte kommen aus `values` per bind.
    pub fn parameterized(&self) -> String {
        format!("{} VALUES ({})", self.head(), vec!["?"; self.values.len()].join(", "))
    }

    fn head(&self) -> String {
        let verb = match (self.replace, self.ignore) {
            (true, _) => "REPLACE INTO",
            (false, true) => "INSERT IGNORE INTO",
            (false, false) => "INSERT INTO",
        };
        let cols = match &self.columns {
            Some(c) => format!(" ({})", c.iter().map(|n| format!("`{n}`")).collect::<Vec<_>>().join(", ")),
            None => String::new(),
        };
        format!("{verb} player.item_proto{cols}")
    }

    /// Kanonischer SQL-Text (für die Server-Dateien), neu aus den geprüften Werten gebaut.
    pub fn to_sql(&self) -> String {
        let lits: Vec<String> = self.values.iter().map(literal_text).collect();
        format!("{} VALUES ({})", self.head(), lits.join(", "))
    }
}

fn literal_text(v: &Value) -> String {
    match v {
        Value::Null => "NULL".into(),
        Value::Int(n) => n.to_string(),
        Value::UInt(n) => n.to_string(),
        Value::Dec(d) => d.clone(),
        Value::Str(s) => format!("'{}'", s.replace('\\', "\\\\").replace('\'', "''").replace('\0', "\\0")),
        Value::Bytes(b) => format!("x'{}'", b.iter().map(|x| format!("{x:02X}")).collect::<String>()),
    }
}

struct Cursor<'a> {
    s: &'a [u8],
    text: &'a str,
    i: usize,
}

impl<'a> Cursor<'a> {
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }
    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }
    fn eat(&mut self, c: u8) -> bool {
        if self.peek() == Some(c) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    /// Bezeichner-Zeichen `[A-Za-z0-9_]`.
    fn word(&mut self) -> &'a str {
        let st = self.i;
        while self.i < self.s.len() && (self.s[self.i].is_ascii_alphanumeric() || self.s[self.i] == b'_') {
            self.i += 1;
        }
        &self.text[st..self.i]
    }
    fn keyword(&mut self, kw: &str) -> bool {
        let save = self.i;
        let w = self.word();
        if w.eq_ignore_ascii_case(kw) {
            true
        } else {
            self.i = save;
            false
        }
    }
    /// Bezeichner, bloß oder in Backticks.
    fn ident(&mut self) -> Result<String, String> {
        if self.eat(b'`') {
            let w = self.word().to_string();
            if w.is_empty() || !self.eat(b'`') {
                return Err("ungültiger Bezeichner in Backticks".into());
            }
            Ok(w)
        } else {
            let w = self.word().to_string();
            if w.is_empty() || w.as_bytes()[0].is_ascii_digit() {
                return Err("Bezeichner erwartet".into());
            }
            Ok(w)
        }
    }
    fn need_ws(&mut self) -> Result<(), String> {
        let before = self.i;
        self.ws();
        if self.i == before {
            Err("Leerzeichen erwartet".into())
        } else {
            Ok(())
        }
    }

    fn string(&mut self) -> Result<String, String> {
        // aktuelles Zeichen ist das öffnende '
        self.i += 1;
        let mut out = Vec::new();
        loop {
            let c = *self.s.get(self.i).ok_or("String nicht beendet")?;
            self.i += 1;
            match c {
                b'\'' => {
                    if self.peek() == Some(b'\'') {
                        self.i += 1;
                        out.push(b'\'');
                    } else {
                        break;
                    }
                }
                b'\\' => {
                    let e = *self.s.get(self.i).ok_or("String nicht beendet")?;
                    self.i += 1;
                    match e {
                        b'0' => out.push(0),
                        b'b' => out.push(8),
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'Z' => out.push(26),
                        b'%' | b'_' => {
                            out.push(b'\\');
                            out.push(e);
                        }
                        other => out.push(other),
                    }
                }
                other => out.push(other),
            }
        }
        String::from_utf8(out).map_err(|_| "String ist kein gültiges UTF-8".to_string())
    }

    fn hex(digits: &str) -> Result<Vec<u8>, String> {
        if !digits.len().is_multiple_of(2) || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("ungültiges Hex-Literal".into());
        }
        Ok((0..digits.len()).step_by(2).map(|k| u8::from_str_radix(&digits[k..k + 2], 16).unwrap()).collect())
    }

    fn value(&mut self) -> Result<Value, String> {
        match self.peek().ok_or("Wert erwartet")? {
            b'\'' => Ok(Value::Str(self.string()?)),
            b'-' | b'0'..=b'9' => {
                let st = self.i;
                if self.s[self.i..].starts_with(b"0x") || self.s[self.i..].starts_with(b"0X") {
                    self.i += 2;
                    let st2 = self.i;
                    while self.peek().is_some_and(|b| b.is_ascii_hexdigit()) {
                        self.i += 1;
                    }
                    return Ok(Value::Bytes(Self::hex(&self.text[st2..self.i])?));
                }
                self.eat(b'-');
                let ds = self.i;
                while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                    self.i += 1;
                }
                if self.i == ds {
                    return Err("ungültige Zahl".into());
                }
                let mut dec = false;
                if self.peek() == Some(b'.') {
                    self.i += 1;
                    let fs = self.i;
                    while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                        self.i += 1;
                    }
                    if self.i == fs {
                        return Err("ungültige Zahl".into());
                    }
                    dec = true;
                }
                // danach muss ein Trenner folgen (keine Buchstaben, Exponenten oder Operatoren)
                if self.peek().is_some_and(|b| !(b.is_ascii_whitespace() || b == b',' || b == b')')) {
                    return Err("ungültige Zahl".into());
                }
                let t = &self.text[st..self.i];
                if dec {
                    Ok(Value::Dec(t.to_string()))
                } else if let Ok(n) = t.parse::<i64>() {
                    Ok(Value::Int(n))
                } else if let Ok(n) = t.parse::<u64>() {
                    Ok(Value::UInt(n))
                } else {
                    Ok(Value::Dec(t.to_string()))
                }
            }
            b'x' | b'X' if self.s.get(self.i + 1) == Some(&b'\'') => {
                self.i += 2;
                let st = self.i;
                while self.peek().is_some_and(|b| b != b'\'') {
                    self.i += 1;
                }
                let d = &self.text[st..self.i];
                if !self.eat(b'\'') {
                    return Err("Hex-Literal nicht beendet".into());
                }
                Ok(Value::Bytes(Self::hex(d)?))
            }
            _ => {
                if self.word().eq_ignore_ascii_case("NULL") {
                    Ok(Value::Null)
                } else {
                    Err("nur Zahlen, 'Strings', x'Hex' und NULL sind als Werte erlaubt".into())
                }
            }
        }
    }
}

/// Prüft ein einzelnes Statement streng und zerlegt es. Erlaubt ist nur
/// `REPLACE INTO` oder `INSERT [IGNORE] INTO` auf `item_proto` (optional `player.`),
/// mit optionaler Spaltenliste und genau einem VALUES-Tupel aus Literalen.
/// Kommentare, weitere Statements oder Text nach dem Tupel werden abgelehnt.
pub fn parse_statement(stmt: &str) -> Result<ProtoStatement, String> {
    let mut c = Cursor { s: stmt.as_bytes(), text: stmt, i: 0 };
    c.ws();
    let mut ignore = false;
    let replace = if c.keyword("REPLACE") {
        true
    } else if c.keyword("INSERT") {
        c.need_ws()?;
        ignore = c.keyword("IGNORE");
        false
    } else {
        return Err("nur REPLACE INTO oder INSERT [IGNORE] INTO sind erlaubt".into());
    };
    c.ws();
    if !c.keyword("INTO") {
        return Err("INTO erwartet".into());
    }
    c.ws();
    let mut table = c.ident()?;
    if c.eat(b'.') {
        if !table.eq_ignore_ascii_case("player") {
            return Err("nur die Datenbank player ist erlaubt".into());
        }
        table = c.ident()?;
    }
    if !table.eq_ignore_ascii_case("item_proto") {
        return Err("nur die Tabelle item_proto ist erlaubt".into());
    }
    c.ws();
    let mut columns = None;
    if c.eat(b'(') {
        let mut cols = Vec::new();
        loop {
            c.ws();
            cols.push(c.ident()?);
            c.ws();
            if c.eat(b',') {
                continue;
            }
            if c.eat(b')') {
                break;
            }
            return Err("Spaltenliste ungültig".into());
        }
        columns = Some(cols);
        c.ws();
    }
    if !c.keyword("VALUES") {
        return Err("VALUES erwartet".into());
    }
    c.ws();
    if !c.eat(b'(') {
        return Err("( nach VALUES erwartet".into());
    }
    let mut values = Vec::new();
    loop {
        c.ws();
        values.push(c.value()?);
        c.ws();
        if c.eat(b',') {
            continue;
        }
        if c.eat(b')') {
            break;
        }
        return Err("Wertliste ungültig (Kommentare, Ausdrücke und Funktionen sind nicht erlaubt)".into());
    }
    c.ws();
    if c.i != c.s.len() {
        return Err("nach dem VALUES-Tupel darf nichts mehr stehen".into());
    }
    if let Some(cols) = &columns {
        if cols.len() != values.len() {
            return Err("Anzahl der Spalten und Werte passt nicht".into());
        }
    }
    let st = ProtoStatement { replace, ignore, columns, values };
    st.vnum()?;
    Ok(st)
}

/// Alle Statements der Datei streng prüfen und nach Vnum filtern; geliefert wird
/// der kanonische, aus den geprüften Werten neu gebaute Text. Ein ungültiges
/// Statement bricht mit deutscher Meldung ab (auch wenn es nicht gewählt ist).
pub fn filter_statements(sql: &str, wanted: &dyn Fn(u32) -> bool) -> Result<Vec<(u32, String)>, String> {
    let mut out = Vec::new();
    for (n, raw) in split_statements(sql).into_iter().enumerate() {
        let st = parse_statement(&raw).map_err(|e| {
            let head: String = raw.chars().take(60).collect();
            format!("Ungültiges SQL-Statement Nr. {} im Paket ({e}): {head}", n + 1)
        })?;
        let v = st.vnum()?;
        if wanted(v) {
            out.push((v, st.to_sql()));
        }
    }
    Ok(out)
}

/// Bindet die Werte eines geprüften Statements an eine sqlx-Abfrage.
pub fn bind_values<'q>(stmt: &'q str, values: &'q [Value]) -> sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments> {
    let mut q = sqlx::query(sqlx::AssertSqlSafe(stmt));
    for v in values {
        q = match v {
            Value::Null => q.bind(None::<String>),
            Value::Int(n) => q.bind(*n),
            Value::UInt(n) => q.bind(*n),
            Value::Dec(d) => q.bind(d.as_str()),
            Value::Str(s) => q.bind(s.as_str()),
            Value::Bytes(b) => q.bind(b.as_slice()),
        };
    }
    q
}

/// Führt ein (bereits kanonisches) Statement geprüft und parametrisiert aus.
pub async fn execute_checked(conn: &mut sqlx::MySqlConnection, canonical: &str) -> Result<(), String> {
    let st = parse_statement(canonical)?;
    let text = st.parameterized();
    bind_values(&text, &st.values).execute(conn).await.map_err(|e| e.to_string())?;
    Ok(())
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

    const SAMPLE: &str = "-- kopf\n-- noch einer\nREPLACE INTO player.item_proto (`vnum`, `name`, `locale_name`) VALUES (10, x'41', 'a;b');\nREPLACE INTO player.item_proto (`vnum`, `name`, `locale_name`) VALUES (11, x'42', 'it''s');\n";

    #[test]
    fn splits_and_ignores_comments_and_quoted_semicolons() {
        let s = split_statements(SAMPLE);
        assert_eq!(s.len(), 2);
        assert!(s[0].contains("'a;b'"));
        assert!(s[1].contains("'it''s'"));
    }

    #[test]
    fn vnum_and_filter() {
        let f = filter_statements(SAMPLE, &|v| v == 11).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].0, 11);
        assert_eq!(f[0].1, "REPLACE INTO player.item_proto (`vnum`, `name`, `locale_name`) VALUES (11, x'42', 'it''s')");
        assert_eq!(filter_statements(SAMPLE, &|_| true).unwrap().len(), 2);
    }

    #[test]
    fn legit_variants() {
        for s in [
            "REPLACE INTO player.item_proto VALUES (1, 'a', NULL, -5, 0.25, x'00ff')",
            "replace into `player`.`item_proto` (`vnum`,`name`) values (1,'it''s')",
            "INSERT IGNORE INTO item_proto (vnum) VALUES (1)",
            "INSERT INTO player.item_proto (vnum, name) VALUES ( 1 , 'a\\'b;c' )",
            "REPLACE INTO player.item_proto (`name`, `vnum`) VALUES (0x01, 4)",
            "REPLACE INTO player.item_proto (name, vnum) VALUES ('x', 7)",
        ] {
            assert!(parse_statement(s).is_ok(), "{s}");
        }
        assert_eq!(parse_statement("REPLACE INTO item_proto (name, vnum) VALUES ('x', 7)").unwrap().vnum(), Ok(7));
    }

    #[test]
    fn rejects_attacks() {
        for s in [
            "UPDATE account.account SET password='x' WHERE 1 -- VALUES (300",
            "UPDATE account.account SET a=1 VALUES (300",
            "REPLACE INTO account.account VALUES (1)",
            "REPLACE INTO player.player VALUES (1)",
            "REPLACE INTO player.item_proto2 VALUES (1)",
            "REPLACE INTO mysql.item_proto VALUES (1)",
            "REPLACE INTO player.item_proto VALUES (1) -- x",
            "REPLACE INTO player.item_proto VALUES (1) # x",
            "REPLACE INTO player.item_proto VALUES (1) /* x */",
            "REPLACE INTO player.item_proto VALUES (1 /* x */)",
            "REPLACE INTO player.item_proto VALUES (1), (2)",
            "REPLACE INTO player.item_proto VALUES (1) SELECT 1",
            "REPLACE INTO player.item_proto VALUES (1); DROP TABLE x",
            "REPLACE INTO player.item_proto VALUES (1, (SELECT 1))",
            "REPLACE INTO player.item_proto VALUES (1, 1+1)",
            "REPLACE INTO player.item_proto VALUES (1, SLEEP(5))",
            "REPLACE INTO player.item_proto VALUES (1, @x)",
            "REPLACE INTO player.item_proto VALUES (1, 1e5)",
            "REPLACE INTO player.item_proto VALUES (1, 'a' 'b')",
            "REPLACE INTO player.item_proto VALUES (1, 'nicht beendet)",
            "REPLACE INTO player.item_proto VALUES (1, x'0')",
            "REPLACE INTO player.item_proto (vnum, name) VALUES (1)",
            "REPLACE INTO player.item_proto (vnum, `na`me) VALUES (1, 2)",
            "REPLACE INTO player.item_proto (vnum, name; DROP) VALUES (1, 2)",
            "REPLACE INTO player.item_proto VALUES ('a', 1)",
            "REPLACE INTO player.item_proto VALUES (-1)",
            "REPLACE IGNORE INTO player.item_proto VALUES (1)",
            "INSERT INTO player.item_proto SELECT * FROM account.account",
            "REPLACE INTO player.item_proto VALUES (1) ON DUPLICATE KEY UPDATE x=1",
            "REPLACE INTO player.item_proto (name) VALUES ('x')",
            "",
        ] {
            assert!(parse_statement(s).is_err(), "sollte abgelehnt werden: {s}");
        }
    }

    #[test]
    fn smuggled_statement_aborts_whole_file() {
        let sql = "REPLACE INTO player.item_proto (vnum) VALUES (1);\nUPDATE account.account SET a=1 -- VALUES (300;\n";
        let e = filter_statements(sql, &|_| true).unwrap_err();
        assert!(e.contains("Nr. 2"), "{e}");
    }

    #[test]
    fn rebuilt_text_and_params() {
        let st = parse_statement("REPLACE INTO item_proto (vnum, name) VALUES (5, 'a\\'b\\\\c''d')").unwrap();
        assert_eq!(st.values[1], Value::Str("a'b\\c'd".into()));
        assert_eq!(st.parameterized(), "REPLACE INTO player.item_proto (`vnum`, `name`) VALUES (?, ?)");
        assert_eq!(st.to_sql(), "REPLACE INTO player.item_proto (`vnum`, `name`) VALUES (5, 'a''b\\\\c''d')");
        assert_eq!(parse_statement(&st.to_sql()).unwrap().values, st.values);
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
