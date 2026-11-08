//! Gemeinsame Bausteine des Mob Drop Editors (wie `components/shared.tsx`),
//! plus kleine Helfer für Zahlenfelder, die auch Drop-Generator und
//! Kisten-Editor nutzen.

use crate::components::Icon;
use leptos::html::Input;
use leptos::prelude::*;

/// Datei-Wert ÷ 4 = reale Drop-Chance (siehe `shared::drop_chance`), für
/// echte 100% braucht es also den Datei-Wert 400 - nicht 100.
pub const MAX_DROP_PERCENT: f64 = 400.0;
/// `max`-Attribut der Prozentfelder (= [`MAX_DROP_PERCENT`]).
pub const MAX_PERCENT_ATTR: &str = "400";

/// `Math.round` aus JS (rundet .5 immer nach oben, auch bei negativen Zahlen).
pub fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

pub fn clamp_percent(value: f64) -> f64 {
    0f64.max(MAX_DROP_PERCENT.min(js_round(value * 10000.0) / 10000.0))
}

/// `Number(s)` aus JS: leer/Leerzeichen = 0, sonst Zahl oder NaN.
pub fn js_number(s: &str) -> f64 {
    let t = s.trim();
    if t.is_empty() {
        return 0.0;
    }
    match t {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    if t.chars().any(|c| c.is_ascii_alphabetic() && c != 'e' && c != 'E') {
        return f64::NAN;
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

/// `Number(s) || fallback` aus JS (NaN und 0 fallen auf `fallback` zurück).
pub fn js_number_or(s: &str, fallback: f64) -> f64 {
    let n = js_number(s);
    if n.is_nan() || n == 0.0 {
        fallback
    } else {
        n
    }
}

/// `String(n)` aus JS für normale Zahlen: 1 -> "1", 0.5 -> "0.5".
pub fn fmt_num(n: f64) -> String {
    if n == 0.0 {
        return "0".into();
    }
    if n.is_nan() {
        return "NaN".into();
    }
    if n.is_infinite() {
        return if n > 0.0 { "Infinity".into() } else { "-Infinity".into() };
    }
    format!("{n}")
}

#[component]
pub fn Field(#[prop(into)] label: Signal<String>, children: Children) -> impl IntoView {
    view! {
        <label class="flex flex-col gap-1 text-xs text-muted-foreground">
            {move || label.get()}
            {children()}
        </label>
    }
}

#[component]
pub fn Modal(
    #[prop(into)] on_close: Callback<()>,
    #[prop(into, optional)] width_class: Option<String>,
    children: Children,
) -> impl IntoView {
    let width = width_class.unwrap_or_else(|| "w-96".into());
    view! {
        <div class="fixed inset-0 flex items-center justify-center bg-black/50">
            <div class=format!("{width} rounded-lg border border-border bg-card p-4")>
                <div class="mb-2 flex justify-end">
                    <button on:click=move |_| on_close.run(()) class="text-muted-foreground hover:text-foreground">
                        <Icon name="X" class="size-4"/>
                    </button>
                </div>
                {children()}
            </div>
        </div>
    }
}

/// `<input type="number">` mit kontrolliertem Wert wie in React: der Wert
/// im Feld wird nur überschrieben, wenn er sich *numerisch* vom Zustand
/// unterscheidet - sonst gingen Zwischenstände wie "0." beim Tippen verloren.
/// `None` steht für ein leeres Feld (`value={x ?? ""}`).
#[component]
pub fn NumInput(
    #[prop(into)] value: Signal<Option<f64>>,
    #[prop(into)] on_input: Callback<String>,
    #[prop(into, optional)] class: String,
    #[prop(into, optional)] min: Option<String>,
    #[prop(into, optional)] max: Option<String>,
    #[prop(into, optional)] step: Option<String>,
    #[prop(optional)] autofocus: bool,
) -> impl IntoView {
    let node = NodeRef::<Input>::new();
    Effect::new(move |_| {
        let v = value.get();
        if let Some(el) = node.get() {
            let dom = el.value();
            let needs_update = match v {
                None => !dom.is_empty(),
                Some(v) => (v == 0.0 && dom.is_empty()) || js_number(&dom) != v,
            };
            if needs_update {
                el.set_value(&v.map(fmt_num).unwrap_or_default());
            }
        }
    });
    view! {
        <input
            node_ref=node
            type="number"
            min=min
            max=max
            step=step
            autofocus=autofocus
            on:input=move |e| on_input.run(event_target_value(&e))
            class=class
        />
    }
}

/// Einfaches `Some(x)` als Signal für [`NumInput`].
pub fn num_sig(f: impl Fn() -> f64 + Send + Sync + 'static) -> Signal<Option<f64>> {
    Signal::derive(move || Some(f()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp() {
        assert_eq!(clamp_percent(-5.0), 0.0);
        assert_eq!(clamp_percent(500.0), 400.0);
        assert_eq!(clamp_percent(1.234567), 1.2346);
        assert_eq!(clamp_percent(0.00005), 0.0001);
    }

    #[test]
    fn numbers() {
        assert_eq!(js_number(""), 0.0);
        assert_eq!(js_number(" 12 "), 12.0);
        assert!(js_number("abc").is_nan());
        assert!(js_number("inf").is_nan());
        assert_eq!(js_number("1e3"), 1000.0);
        assert_eq!(js_number_or("", 1.0), 1.0);
        assert_eq!(js_number_or("0", 1.0), 1.0);
        assert_eq!(fmt_num(101.0), "101");
        assert_eq!(fmt_num(0.25), "0.25");
        assert_eq!(fmt_num(-0.0), "0");
    }
}
