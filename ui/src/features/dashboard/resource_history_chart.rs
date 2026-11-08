//! Minimales, selbst gezeichnetes SVG-Liniendiagramm (CPU/RAM-Verlauf) -
//! keine Diagramm-Bibliothek, wie im alten Frontend.

use crate::i18n::{t, t_with};
use crate::lib::format::de_time;
use crate::tauri::invoke;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ResourceHistoryPoint {
    pub id: i64,
    pub created_at: String,
    pub cpu_percent: f64,
    pub ram_used_bytes: Option<f64>,
    pub ram_total_bytes: Option<f64>,
    pub disk_capacity_percent: Option<f64>,
}

const POINTS_TO_FETCH: u32 = 200;
const REFRESH_MS: u64 = 60_000;
const CHART_WIDTH: f64 = 600.0;
const CHART_HEIGHT: f64 = 120.0;

/// Formatiert eine Zahl wie `String(n)` in JS (ganze Zahlen ohne ".0").
fn js_num(n: f64) -> String {
    if n.fract() == 0.0 && n.is_finite() {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

/// Bildet eine Reihe von 0-100-Werten auf einen SVG-`points`-String ab.
pub fn to_polyline(values: &[Option<f64>], width: f64, height: f64) -> String {
    if values.iter().all(|v| v.is_none()) {
        return String::new();
    }
    let step = if values.len() > 1 { width / (values.len() - 1) as f64 } else { 0.0 };
    values
        .iter()
        .enumerate()
        .filter_map(|(i, v)| {
            v.map(|v| format!("{},{}", js_num(i as f64 * step), js_num(height - (v.clamp(0.0, 100.0) / 100.0) * height)))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// RAM-Belegung in Prozent, falls beide Werte vorhanden sind (und total ≠ 0).
pub fn ram_percent(p: &ResourceHistoryPoint) -> Option<f64> {
    match (p.ram_used_bytes, p.ram_total_bytes) {
        (Some(used), Some(total)) if total != 0.0 => Some(used / total * 100.0),
        _ => None,
    }
}

#[component]
pub fn ResourceHistoryChart() -> impl IntoView {
    let points = RwSignal::new(None::<Vec<ResourceHistoryPoint>>);
    let error = RwSignal::new(None::<String>);
    let cancelled = Arc::new(AtomicBool::new(false));

    let load = {
        let cancelled = cancelled.clone();
        move || {
            let cancelled = cancelled.clone();
            spawn_local(async move {
                let res = invoke::<Vec<ResourceHistoryPoint>>("get_resource_history", &json!({ "limit": POINTS_TO_FETCH })).await;
                if cancelled.load(Ordering::Relaxed) {
                    return;
                }
                match res {
                    Ok(r) => {
                        points.set(Some(r));
                        error.set(None);
                    }
                    Err(e) => error.set(Some(e)),
                }
            });
        }
    };
    load();
    let interval = set_interval_with_handle(load, Duration::from_millis(REFRESH_MS)).ok();
    on_cleanup(move || {
        cancelled.store(true, Ordering::Relaxed);
        if let Some(h) = interval {
            h.clear();
        }
    });

    move || {
        if let Some(e) = error.get() {
            return view! { <p class="text-sm text-destructive">{e}</p> }.into_any();
        }
        let pts = points.get().unwrap_or_default();
        if pts.len() < 2 {
            return view! { <p class="text-sm text-muted-foreground">{t("resourceHistory.notEnoughData")}</p> }.into_any();
        }
        let cpu: Vec<Option<f64>> = pts.iter().map(|p| Some(p.cpu_percent)).collect();
        let ram: Vec<Option<f64>> = pts.iter().map(ram_percent).collect();
        let latest = pts.last().cloned().unwrap();
        let oldest = pts.first().cloned().unwrap();
        let grid = [25.0, 50.0, 75.0]
            .iter()
            .map(|y| {
                let yy = js_num(CHART_HEIGHT - (y / 100.0) * CHART_HEIGHT);
                view! {
                    <line x1="0" x2=js_num(CHART_WIDTH) y1=yy.clone() y2=yy class="stroke-border" stroke-width="1"></line>
                }
            })
            .collect_view();
        let ram_text = ram_percent(&latest).map(|r| format!("{r:.1}%")).unwrap_or_else(|| "-".into());
        view! {
            <div class="space-y-2">
                <svg
                    viewBox=format!("0 0 {} {}", js_num(CHART_WIDTH), js_num(CHART_HEIGHT))
                    preserveAspectRatio="none"
                    class="h-28 w-full rounded-lg border border-border bg-card"
                >
                    {grid}
                    <polyline
                        points=to_polyline(&cpu, CHART_WIDTH, CHART_HEIGHT)
                        fill="none"
                        class="stroke-primary"
                        stroke-width="2"
                        vector-effect="non-scaling-stroke"
                    ></polyline>
                    <polyline
                        points=to_polyline(&ram, CHART_WIDTH, CHART_HEIGHT)
                        fill="none"
                        class="stroke-amber-500"
                        stroke-width="2"
                        vector-effect="non-scaling-stroke"
                    ></polyline>
                </svg>
                <div class="flex flex-wrap items-center justify-between gap-2 text-xs text-muted-foreground">
                    <div class="flex items-center gap-3">
                        <span class="flex items-center gap-1">
                            <span class="size-2 rounded-full bg-primary"></span>
                            {format!("{}: {:.1}%", t("resourceHistory.cpu"), latest.cpu_percent)}
                        </span>
                        <span class="flex items-center gap-1">
                            <span class="size-2 rounded-full bg-amber-500"></span>
                            {format!("{}: {}", t("resourceHistory.ram"), ram_text)}
                        </span>
                    </div>
                    <span>
                        {t_with(
                            "resourceHistory.range",
                            &[("from", &de_time(&oldest.created_at)), ("to", &de_time(&latest.created_at))],
                        )}
                    </span>
                </div>
            </div>
        }
        .into_any()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polyline_maps_values() {
        assert_eq!(to_polyline(&[Some(0.0), Some(100.0)], 600.0, 120.0), "0,120 600,0");
        assert_eq!(to_polyline(&[Some(50.0), None, Some(150.0)], 600.0, 120.0), "0,60 600,0");
        assert_eq!(to_polyline(&[None, None], 600.0, 120.0), "");
        assert_eq!(to_polyline(&[Some(-5.0)], 600.0, 120.0), "0,120");
    }

    #[test]
    fn ram() {
        let mut p = ResourceHistoryPoint {
            id: 1,
            created_at: String::new(),
            cpu_percent: 1.0,
            ram_used_bytes: Some(1.0),
            ram_total_bytes: Some(4.0),
            disk_capacity_percent: None,
        };
        assert_eq!(ram_percent(&p), Some(25.0));
        p.ram_total_bytes = Some(0.0);
        assert_eq!(ram_percent(&p), None);
        p.ram_total_bytes = None;
        assert_eq!(ram_percent(&p), None);
    }
}
