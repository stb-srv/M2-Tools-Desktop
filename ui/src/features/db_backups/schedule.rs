//! Intervall-basierter Zeitplan ("alle N Stunden") statt einer festen
//! Uhrzeit - funktioniert unabhängig davon, ob die App zur exakten Minute
//! offen ist. Es gibt keinen Hintergrunddienst: bei jedem Tick des
//! periodischen Checks (CrashWatch) wird nur geprüft, ob seit dem letzten
//! automatischen Backup genug Zeit vergangen ist.

/// Wählbare Intervalle; 0 = aus.
pub const SCHEDULE_HOURS_OPTIONS: [u32; 5] = [0, 6, 12, 24, 48];

const HOUR_MS: f64 = 60.0 * 60.0 * 1000.0;

/// Wie `Date.parse` - `None` statt `NaN`. Im Browser wird das echte
/// `Date.parse` benutzt; nativ (Tests) ein ISO-8601-Parser.
pub fn parse_date_ms(s: &str) -> Option<f64> {
    #[cfg(target_arch = "wasm32")]
    {
        let ms = js_sys::Date::parse(s);
        if ms.is_nan() {
            None
        } else {
            Some(ms)
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        parse_iso_utc_ms(s)
    }
}

/// ISO-8601 (`YYYY-MM-DD`, `YYYY-MM-DDTHH:MM[:SS[.sss]][Z|±HH:MM]`).
/// Ohne Zeitzone wird UTC angenommen (nur für native Tests gedacht).
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn parse_iso_utc_ms(s: &str) -> Option<f64> {
    let s = s.trim();
    let num = |t: &str| -> Option<i64> {
        if t.is_empty() || !t.chars().all(|c| c.is_ascii_digit()) {
            None
        } else {
            t.parse().ok()
        }
    };
    let (date, rest) = match s.find(['T', ' ']) {
        Some(i) => (&s[..i], &s[i + 1..]),
        None => (s, ""),
    };
    let mut dp = date.split('-');
    let y = num(dp.next()?)?;
    let mo = num(dp.next()?)?;
    let d = num(dp.next()?)?;
    if dp.next().is_some() || !(1..=12).contains(&mo) || !(1..=31).contains(&d) {
        return None;
    }
    let (mut h, mut mi, mut sec, mut ms, mut offset_min) = (0i64, 0i64, 0i64, 0f64, 0i64);
    if !rest.is_empty() {
        let (time, tz) = if let Some(t) = rest.strip_suffix('Z') {
            (t, 0)
        } else if let Some(i) = rest.rfind(['+', '-']) {
            let (t, z) = rest.split_at(i);
            let sign = if z.starts_with('-') { -1 } else { 1 };
            let z = &z[1..];
            let (zh, zm) = z.split_once(':').unwrap_or((&z[..z.len().min(2)], &z[z.len().min(2)..]));
            (t, sign * (num(zh)? * 60 + num(zm)?))
        } else {
            (rest, 0)
        };
        offset_min = tz;
        let mut tp = time.split(':');
        h = num(tp.next()?)?;
        mi = num(tp.next()?)?;
        if let Some(sp) = tp.next() {
            let (sw, frac) = sp.split_once('.').unwrap_or((sp, ""));
            sec = num(sw)?;
            if !frac.is_empty() {
                num(frac)?;
                ms = format!("0.{frac}").parse::<f64>().ok()? * 1000.0;
                ms = ms.floor();
            }
        }
        if h > 24 || mi > 59 || sec > 59 {
            return None;
        }
    }
    // Tage seit 1970-01-01 (Howard Hinnant, days_from_civil).
    let y2 = if mo <= 2 { y - 1 } else { y };
    let era = if y2 >= 0 { y2 } else { y2 - 399 } / 400;
    let yoe = y2 - era * 400;
    let mp = (mo + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    let secs = days * 86400 + h * 3600 + mi * 60 + sec - offset_min * 60;
    Some(secs as f64 * 1000.0 + ms)
}

/// Ob ein neues automatisches Backup fällig ist: Zeitplan aktiv und entweder
/// noch nie eins gelaufen oder das Intervall seit dem letzten verstrichen.
pub fn is_backup_due(now_ms: f64, last_auto_at_iso: Option<&str>, schedule_hours: f64) -> bool {
    if schedule_hours.is_nan() || schedule_hours <= 0.0 {
        return false;
    }
    let Some(last) = last_auto_at_iso.filter(|s| !s.is_empty()) else { return true };
    let Some(last_ms) = parse_date_ms(last) else { return true };
    now_ms - last_ms >= schedule_hours * HOUR_MS
}

/// Nächster Fälligkeitszeitpunkt in ms seit Epoch, oder `None` bei
/// deaktiviertem Zeitplan. Ohne (gültigen) letzten Lauf: `now_ms`.
pub fn next_due_at(now_ms: f64, last_auto_at_iso: Option<&str>, schedule_hours: f64) -> Option<f64> {
    if schedule_hours.is_nan() || schedule_hours <= 0.0 {
        return None;
    }
    let Some(last) = last_auto_at_iso.filter(|s| !s.is_empty()) else { return Some(now_ms) };
    let Some(last_ms) = parse_date_ms(last) else { return Some(now_ms) };
    Some(last_ms + schedule_hours * HOUR_MS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iso(ms: f64) -> String {
        // Rückweg für die Tests: ms -> "YYYY-MM-DDTHH:MM:SS.000Z".
        let secs = (ms / 1000.0).floor() as i64;
        let days = secs.div_euclid(86400);
        let rem = secs.rem_euclid(86400);
        let z = days + 719468;
        let era = if z >= 0 { z } else { z - 146096 } / 146097;
        let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = if m <= 2 { y + 1 } else { y };
        format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.000Z", rem / 3600, rem % 3600 / 60, rem % 60)
    }

    fn now() -> f64 {
        parse_date_ms("2026-08-25T12:00:00.000Z").unwrap()
    }

    #[test]
    fn never_due_when_disabled() {
        assert!(!is_backup_due(now(), None, 0.0));
        assert!(!is_backup_due(now(), Some("2020-01-01T00:00:00.000Z"), 0.0));
    }

    #[test]
    fn due_immediately_if_never_ran() {
        assert!(is_backup_due(now(), None, 24.0));
    }

    #[test]
    fn due_once_interval_elapsed() {
        let exactly_24h_ago = iso(now() - 24.0 * 60.0 * 60.0 * 1000.0);
        assert!(is_backup_due(now(), Some(&exactly_24h_ago), 24.0));
    }

    #[test]
    fn not_due_before_interval_elapses() {
        let one_hour_ago = iso(now() - 60.0 * 60.0 * 1000.0);
        assert!(!is_backup_due(now(), Some(&one_hour_ago), 24.0));
    }

    #[test]
    fn unparseable_timestamp_is_due() {
        assert!(is_backup_due(now(), Some("not-a-date"), 24.0));
    }

    #[test]
    fn next_due_null_when_disabled() {
        assert_eq!(next_due_at(now(), None, 0.0), None);
    }

    #[test]
    fn next_due_now_when_never_ran() {
        assert_eq!(next_due_at(now(), None, 24.0), Some(now()));
    }

    #[test]
    fn next_due_adds_interval() {
        let due = next_due_at(now(), Some("2026-08-25T00:00:00.000Z"), 12.0).unwrap();
        assert_eq!(iso(due), "2026-08-25T12:00:00.000Z");
    }

    #[test]
    fn iso_parser_roundtrip_and_offsets() {
        assert_eq!(parse_date_ms("1970-01-01T00:00:00Z"), Some(0.0));
        assert_eq!(parse_date_ms("1970-01-01T01:00:00+01:00"), Some(0.0));
        assert_eq!(parse_date_ms("2026-08-25T12:00:00.500Z").map(|m| m % 1000.0), Some(500.0));
        assert_eq!(parse_date_ms("garbage"), None);
    }
}
