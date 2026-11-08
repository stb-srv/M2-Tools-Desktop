//! Echte Drop-Chance aus dem `percent`-Wert der Drop-Dateien.
//!
//! Gegen den Server-Quellcode geprüft (item_manager_read_tables.cpp:665-667,
//! item_manager.cpp:664-698 und 867-871): unter Basisbedingungen ergibt sich
//! `percent / 400` als *Anteil* (0..1), also `percent / 4` in Prozent. Eine
//! frühere Version zeigte den Anteil direkt mit "%" an - Faktor 100 daneben.

pub fn real_drop_chance_percent(percent: f64) -> f64 {
    percent / 4.0
}

/// "2.5%", "0.25%", "1%" - zwei Nachkommastellen, Nullen am Ende gekürzt.
pub fn format_real_drop_chance(percent: f64) -> String {
    let fixed = format!("{:.2}", real_drop_chance_percent(percent));
    let trimmed = fixed.trim_end_matches('0').trim_end_matches('.');
    format!("{}%", if trimmed.is_empty() { "0" } else { trimmed })
}

#[derive(Clone, Debug, PartialEq)]
pub struct DropSimulationResult {
    pub item_vnum: i64,
    /// 0..100 - Wahrscheinlichkeit für mindestens einen Drop über alle Kills.
    pub chance_at_least_one: f64,
    /// Erwartete Anzahl Drops über alle Kills.
    pub expected_count: f64,
}

/// Jeder Kill als unabhängiger Bernoulli-Versuch mit derselben Chance, die
/// der Editor als "≈X% real" anzeigt: P(≥1) = 1 - (1-p)^N, E = N·p.
pub fn simulate_drops(items: &[(i64, f64)], kills: f64) -> Vec<DropSimulationResult> {
    let n = if kills.is_finite() { kills.floor().max(0.0) } else { 0.0 };
    items
        .iter()
        .map(|&(item_vnum, percent)| {
            let p = (real_drop_chance_percent(percent) / 100.0).clamp(0.0, 1.0);
            DropSimulationResult {
                item_vnum,
                chance_at_least_one: if n == 0.0 { 0.0 } else { (1.0 - (1.0 - p).powf(n)) * 100.0 },
                expected_count: p * n,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_chance() {
        assert_eq!(real_drop_chance_percent(10.0), 2.5);
        assert_eq!(real_drop_chance_percent(1.0), 0.25);
        assert_eq!(real_drop_chance_percent(100.0), 25.0);
    }

    #[test]
    fn formatting() {
        assert_eq!(format_real_drop_chance(10.0), "2.5%");
        assert_eq!(format_real_drop_chance(1.0), "0.25%");
        assert_eq!(format_real_drop_chance(4.0), "1%");
        assert_eq!(format_real_drop_chance(0.0), "0%");
    }

    #[test]
    fn simulation() {
        let r = simulate_drops(&[(1, 4.0)], 1.0);
        assert!((r[0].expected_count - 0.01).abs() < 1e-10);
        let r = simulate_drops(&[(1, 4.0)], 100.0);
        assert!((r[0].chance_at_least_one - 63.397).abs() < 0.01);
        assert!((r[0].expected_count - 1.0).abs() < 1e-10);
        for k in [0.0, -5.0, f64::NAN] {
            let r = simulate_drops(&[(1, 4.0)], k);
            assert_eq!(r[0].chance_at_least_one, 0.0);
            assert_eq!(r[0].expected_count, 0.0);
        }
        let r = simulate_drops(&[(1, 400.0)], 1.0);
        assert!((r[0].chance_at_least_one - 100.0).abs() < 1e-10);
        let r = simulate_drops(&[(1, 4.0), (2, 40.0)], 100.0);
        assert_eq!(r.len(), 2);
        assert!(r[1].expected_count > r[0].expected_count);
    }
}
