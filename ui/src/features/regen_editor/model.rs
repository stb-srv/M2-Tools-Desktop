//! Datenmodell und reine Logik des Regen-Editors (Zeilen, Pfade, Karten-
//! Ansicht). Die Zahlen sind `f64` wie JS-`number` im bisherigen Frontend.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RegenSpawn {
    pub spawn_type: String,
    pub center_x: f64,
    pub center_y: f64,
    pub radius_x: f64,
    pub radius_y: f64,
    pub z_section: f64,
    pub direction: f64,
    pub regen_time: String,
    pub regen_percent: f64,
    pub max_count: f64,
    pub vnum: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RegenException {
    pub center_x: f64,
    pub center_y: f64,
    pub radius_x: f64,
    pub radius_y: f64,
    pub z_section: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum RegenLine {
    Spawn(RegenSpawn),
    Exception(RegenException),
    Raw { text: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    Spawn,
    Exception,
    Raw,
}

/// Zahlenfelder einer Zeile (für die generischen Eingabefelder).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumKey {
    CenterX,
    CenterY,
    RadiusX,
    RadiusY,
    ZSection,
    Direction,
    MaxCount,
    Vnum,
}

impl RegenLine {
    pub fn kind(&self) -> LineKind {
        match self {
            RegenLine::Spawn(_) => LineKind::Spawn,
            RegenLine::Exception(_) => LineKind::Exception,
            RegenLine::Raw { .. } => LineKind::Raw,
        }
    }

    pub fn num(&self, key: NumKey) -> f64 {
        match self {
            RegenLine::Spawn(s) => match key {
                NumKey::CenterX => s.center_x,
                NumKey::CenterY => s.center_y,
                NumKey::RadiusX => s.radius_x,
                NumKey::RadiusY => s.radius_y,
                NumKey::ZSection => s.z_section,
                NumKey::Direction => s.direction,
                NumKey::MaxCount => s.max_count,
                NumKey::Vnum => s.vnum,
            },
            RegenLine::Exception(e) => match key {
                NumKey::CenterX => e.center_x,
                NumKey::CenterY => e.center_y,
                NumKey::RadiusX => e.radius_x,
                NumKey::RadiusY => e.radius_y,
                NumKey::ZSection => e.z_section,
                _ => 0.0,
            },
            RegenLine::Raw { .. } => 0.0,
        }
    }

    pub fn set_num(&mut self, key: NumKey, v: f64) {
        match self {
            RegenLine::Spawn(s) => {
                let f = match key {
                    NumKey::CenterX => &mut s.center_x,
                    NumKey::CenterY => &mut s.center_y,
                    NumKey::RadiusX => &mut s.radius_x,
                    NumKey::RadiusY => &mut s.radius_y,
                    NumKey::ZSection => &mut s.z_section,
                    NumKey::Direction => &mut s.direction,
                    NumKey::MaxCount => &mut s.max_count,
                    NumKey::Vnum => &mut s.vnum,
                };
                *f = v;
            }
            RegenLine::Exception(e) => {
                let f = match key {
                    NumKey::CenterX => &mut e.center_x,
                    NumKey::CenterY => &mut e.center_y,
                    NumKey::RadiusX => &mut e.radius_x,
                    NumKey::RadiusY => &mut e.radius_y,
                    NumKey::ZSection => &mut e.z_section,
                    _ => return,
                };
                *f = v;
            }
            RegenLine::Raw { .. } => {}
        }
    }

    pub fn spawn_type(&self) -> &str {
        match self {
            RegenLine::Spawn(s) => &s.spawn_type,
            _ => "",
        }
    }
}

pub struct SpawnTypeOption {
    pub value: &'static str,
    pub label: &'static str,
}

pub const SPAWN_TYPES: &[SpawnTypeOption] = &[
    SpawnTypeOption { value: "m", label: "m - Einzelner Mob" },
    SpawnTypeOption { value: "g", label: "g - Gruppe" },
    SpawnTypeOption { value: "ga", label: "ga - Gruppe (aggressiv)" },
    SpawnTypeOption { value: "r", label: "r - Gruppe von Gruppen" },
    SpawnTypeOption { value: "s", label: "s - Irgendwo auf der Map" },
];

pub fn new_spawn() -> RegenLine {
    RegenLine::Spawn(RegenSpawn {
        spawn_type: "m".into(),
        center_x: 0.0,
        center_y: 0.0,
        radius_x: 10.0,
        radius_y: 10.0,
        z_section: 0.0,
        direction: 0.0,
        regen_time: "0s".into(),
        regen_percent: 100.0,
        max_count: 1.0,
        vnum: 0.0,
    })
}

pub fn new_exception() -> RegenLine {
    RegenLine::Exception(RegenException {
        center_x: 0.0,
        center_y: 0.0,
        radius_x: 10.0,
        radius_y: 10.0,
        z_section: 0.0,
    })
}

// ------------------------------------------------------------- Pfade

/// `absolute.slice(baseDir.length).replace(/^\//, "")` (ohne Basis: unverändert).
pub fn to_relative(base_dir: Option<&str>, absolute: &str) -> String {
    let Some(base) = base_dir else { return absolute.to_string() };
    let rest = absolute.get(base.len()..).unwrap_or("");
    rest.strip_prefix('/').unwrap_or(rest).to_string()
}

/// Ein Ordner höher, aber nie über `base_dir` hinaus.
pub fn go_up(prev: Option<&str>, base_dir: Option<&str>) -> Option<String> {
    let (Some(p), Some(base)) = (prev, base_dir) else { return prev.map(str::to_string) };
    if p.len() <= base.len() {
        return Some(p.to_string());
    }
    match p.rfind('/') {
        None => Some(base.to_string()),
        Some(idx) => Some(p[..idx].to_string()),
    }
}

pub fn breadcrumbs(dir: Option<&str>, base_dir: Option<&str>) -> Vec<String> {
    let (Some(d), Some(base)) = (dir, base_dir) else { return vec![] };
    if d.is_empty() || base.is_empty() || d == base {
        return vec![];
    }
    to_relative(Some(base), d).split('/').map(str::to_string).collect()
}

// ------------------------------------------------------------- Karte

pub struct FilterKind {
    pub key: &'static str,
    pub label: &'static str,
    pub color: &'static str,
}

// Spawn-Typ ist die erste Zeichen-gesteuerte Kennung (siehe regen.rs:
// m/g/r/s, zweites Zeichen 'a' = aggressiv) - fürs Filtern/Einfärben reicht
// das erste Zeichen, statt jede exotische Kombination einzeln zu kennen.
pub const FILTER_KINDS: &[FilterKind] = &[
    FilterKind { key: "m", label: "Einzelner Mob", color: "#38bdf8" },
    FilterKind { key: "g", label: "Gruppe", color: "#a3e635" },
    FilterKind { key: "r", label: "Gruppe von Gruppen", color: "#c084fc" },
    FilterKind { key: "s", label: "Überall", color: "#facc15" },
];

pub fn color_for_spawn_type(spawn_type: &str) -> &'static str {
    FILTER_KINDS
        .iter()
        .find(|k| spawn_type.starts_with(k.key))
        .map(|k| k.color)
        .unwrap_or("#94a3b8")
}

/// `spawn_type.charAt(0)`
pub fn first_char(s: &str) -> String {
    s.chars().next().map(String::from).unwrap_or_default()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewBox {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl ViewBox {
    pub fn attr(&self) -> String {
        format!("{} {} {} {}", self.x, self.y, self.w, self.h)
    }
}

/// Mausrad-Zoom um den Punkt `(px, py)` (SVG-Koordinaten).
pub fn zoom(vb: ViewBox, px: f64, py: f64, delta_y: f64, map_width_px: f64) -> ViewBox {
    let factor = if delta_y > 0.0 { 1.15 } else { 1.0 / 1.15 };
    let min_w = map_width_px / 20.0;
    let max_w = map_width_px * 4.0;
    let new_w = max_w.min(min_w.max(vb.w * factor));
    let ratio = new_w / vb.w;
    let new_h = vb.h * ratio;
    ViewBox { x: px - (px - vb.x) * ratio, y: py - (py - vb.y) * ratio, w: new_w, h: new_h }
}

/// 2D-Affinmatrix wie `DOMMatrix` (a b c d e f).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl Affine {
    pub fn inverse(&self) -> Option<Affine> {
        let det = self.a * self.d - self.b * self.c;
        if det == 0.0 || !det.is_finite() {
            return None;
        }
        let a = self.d / det;
        let b = -self.b / det;
        let c = -self.c / det;
        let d = self.a / det;
        let e = -(a * self.e + c * self.f);
        let f = -(b * self.e + d * self.f);
        Some(Affine { a, b, c, d, e, f })
    }

    pub fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        (self.a * x + self.c * y + self.e, self.b * x + self.d * y + self.f)
    }
}

/// `Math.round` aus JS (x.5 rundet Richtung +Unendlich).
pub fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_shape_matches_backend() {
        let json = serde_json::to_value(new_exception()).unwrap();
        assert_eq!(json["kind"], "Exception");
        assert_eq!(json["radius_x"], 10.0);
        let raw: RegenLine = serde_json::from_str(r#"{"kind":"Raw","text":"// x"}"#).unwrap();
        assert_eq!(raw, RegenLine::Raw { text: "// x".into() });
        let spawn: RegenLine = serde_json::from_str(
            r#"{"kind":"Spawn","spawn_type":"ga","center_x":210,"center_y":210,"radius_x":10,"radius_y":10,"z_section":0,"direction":0,"regen_time":"0s","regen_percent":100,"max_count":1,"vnum":1024}"#,
        )
        .unwrap();
        assert_eq!(spawn.num(NumKey::Vnum), 1024.0);
        assert_eq!(spawn.spawn_type(), "ga");
    }

    #[test]
    fn set_num_only_touches_existing_fields() {
        let mut e = new_exception();
        e.set_num(NumKey::Direction, 5.0);
        assert_eq!(e, new_exception());
        e.set_num(NumKey::CenterX, 5.0);
        assert_eq!(e.num(NumKey::CenterX), 5.0);
    }

    #[test]
    fn paths() {
        let base = Some("/usr/home/game/share");
        assert_eq!(to_relative(base, "/usr/home/game/share/data/a.txt"), "data/a.txt");
        assert_eq!(to_relative(None, "/x/y"), "/x/y");
        assert_eq!(go_up(Some("/usr/home/game/share/data/dungeon"), base).as_deref(), Some("/usr/home/game/share/data"));
        assert_eq!(go_up(Some("/usr/home/game/share"), base).as_deref(), Some("/usr/home/game/share"));
        assert_eq!(go_up(None, base), None);
        assert_eq!(breadcrumbs(Some("/usr/home/game/share/data/dungeon"), base), vec!["data", "dungeon"]);
        assert!(breadcrumbs(Some("/usr/home/game/share"), base).is_empty());
    }

    #[test]
    fn colors_and_filter() {
        assert_eq!(color_for_spawn_type("ga"), "#a3e635");
        assert_eq!(color_for_spawn_type("m"), "#38bdf8");
        assert_eq!(color_for_spawn_type(""), "#94a3b8");
        assert_eq!(color_for_spawn_type("x"), "#94a3b8");
        assert_eq!(first_char("ga"), "g");
        assert_eq!(first_char(""), "");
    }

    #[test]
    fn zoom_clamps_and_keeps_point() {
        let vb = ViewBox { x: 0.0, y: 0.0, w: 100.0, h: 50.0 };
        let z = zoom(vb, 50.0, 25.0, -1.0, 100.0);
        assert!((z.w - 100.0 / 1.15).abs() < 1e-9);
        assert!((z.h - 50.0 / 1.15).abs() < 1e-9);
        // Punkt unter der Maus bleibt an derselben relativen Stelle
        assert!(((50.0 - z.x) / z.w - 0.5).abs() < 1e-9);
        let mut v = vb;
        for _ in 0..100 {
            v = zoom(v, 0.0, 0.0, -1.0, 100.0);
        }
        assert!((v.w - 5.0).abs() < 1e-9);
        for _ in 0..100 {
            v = zoom(v, 0.0, 0.0, 1.0, 100.0);
        }
        assert!((v.w - 400.0).abs() < 1e-9);
    }

    #[test]
    fn affine_inverse() {
        let m = Affine { a: 2.0, b: 0.0, c: 0.0, d: 2.0, e: 10.0, f: 20.0 };
        let inv = m.inverse().unwrap();
        let (x, y) = m.apply(3.0, 4.0);
        let (bx, by) = inv.apply(x, y);
        assert!((bx - 3.0).abs() < 1e-12 && (by - 4.0).abs() < 1e-12);
        assert!(Affine { a: 0.0, b: 0.0, c: 0.0, d: 0.0, e: 0.0, f: 0.0 }.inverse().is_none());
    }

    #[test]
    fn rounding_like_js() {
        assert_eq!(js_round(2.5), 3.0);
        assert_eq!(js_round(-2.5), -2.0);
        assert_eq!(js_round(-2.6), -3.0);
    }
}
