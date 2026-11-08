//! Zahlen-/Datumsformatierung wie `toLocaleString("de-DE")`.

/// Tausenderpunkte: 1234567 -> "1.234.567".
pub fn de_int(n: i64) -> String {
    let neg = n < 0;
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    if neg {
        format!("-{out}")
    } else {
        out
    }
}

/// Dezimalzahl mit Komma und fester Nachkommastellenzahl: 1234.5 -> "1.234,50".
pub fn de_float(x: f64, decimals: usize) -> String {
    let s = format!("{:.*}", decimals, x.abs());
    let (int, frac) = s.split_once('.').unwrap_or((&s, ""));
    let int = de_int(int.parse::<i64>().unwrap_or(0));
    let sign = if x < 0.0 && x.abs() >= 0.5 * 10f64.powi(-(decimals as i32)) { "-" } else { "" };
    if frac.is_empty() {
        format!("{sign}{int}")
    } else {
        format!("{sign}{int},{frac}")
    }
}

/// Datum/Zeit aus einem ISO-/SQL-Zeitstempel im lokalen Format "dd.mm.yyyy, hh:mm:ss"
/// (wie `new Date(x).toLocaleString("de-DE")`).
pub fn de_datetime(iso: &str) -> String {
    let d = js_sys::Date::new(&iso.into());
    if d.get_time().is_nan() {
        return iso.to_string();
    }
    format!(
        "{:02}.{:02}.{}, {:02}:{:02}:{:02}",
        d.get_date(),
        d.get_month() + 1,
        d.get_full_year(),
        d.get_hours(),
        d.get_minutes(),
        d.get_seconds()
    )
}

/// Nur das Datum: "dd.mm.yyyy".
pub fn de_date(iso: &str) -> String {
    let d = js_sys::Date::new(&iso.into());
    if d.get_time().is_nan() {
        return iso.to_string();
    }
    format!("{:02}.{:02}.{}", d.get_date(), d.get_month() + 1, d.get_full_year())
}

/// Nur die Uhrzeit: "hh:mm:ss".
pub fn de_time(iso: &str) -> String {
    let d = js_sys::Date::new(&iso.into());
    if d.get_time().is_nan() {
        return iso.to_string();
    }
    format!("{:02}:{:02}:{:02}", d.get_hours(), d.get_minutes(), d.get_seconds())
}

/// Dateigröße menschenlesbar (B, KB, MB, GB).
pub fn bytes(n: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut v = n as f64;
    let mut u = 0;
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{n} B")
    } else {
        format!("{} {}", de_float(v, 1), UNITS[u])
    }
}

/// Aktuelle Zeit in Millisekunden seit Epoch (wie `Date.now()`).
pub fn now_ms() -> f64 {
    js_sys::Date::now()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ints() {
        assert_eq!(de_int(0), "0");
        assert_eq!(de_int(999), "999");
        assert_eq!(de_int(1000), "1.000");
        assert_eq!(de_int(-1234567), "-1.234.567");
    }

    #[test]
    fn floats() {
        assert_eq!(de_float(1234.5, 2), "1.234,50");
        assert_eq!(de_float(0.125, 1), "0,1");
    }
}
