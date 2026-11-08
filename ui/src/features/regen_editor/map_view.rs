//! Karten-Ansicht: alternative Darstellung derselben Regen-Zeilen über der
//! Minimap des Clients (SVG). Zoom per Mausrad, Verschieben per Ziehen,
//! Marker verschieben/skalieren, Spawn per Klick hinzufügen.

use super::line_row::RegenLineRow;
use super::model::{
    color_for_spawn_type, first_char, js_round, new_spawn, zoom, Affine, LineKind, RegenLine, ViewBox, FILTER_KINDS,
};
use crate::components::button::button_class;
use crate::components::{Button, ButtonSize, ButtonVariant, Icon};
use leptos::prelude::*;
use serde::Deserialize;
use std::collections::HashSet;
use wasm_bindgen::JsCast;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct MapFolderInfo {
    pub category: String,
    pub folder_name: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct RegenMapImage {
    pub image_data_url: String,
    pub units_per_pixel: f64,
    pub width_px: f64,
    pub height_px: f64,
}

#[derive(Clone, Copy)]
enum DragState {
    Pan { inverse: Affine, start: (f64, f64), start_view_box: ViewBox },
    Move { inverse: Affine, start: (f64, f64), index: usize, original_x: f64, original_y: f64 },
    Resize { inverse: Affine, start: (f64, f64), index: usize, original_radius_x: f64, original_radius_y: f64 },
}

#[component]
pub fn RegenMapView(
    #[prop(into)] lines: Signal<Vec<RegenLine>>,
    #[prop(into)] map_image: Signal<RegenMapImage>,
    #[prop(into)] on_update_line: Callback<(usize, RegenLine)>,
    #[prop(into)] on_add_line: Callback<RegenLine>,
    #[prop(into)] on_remove_line: Callback<usize>,
    /// Nur sichtbar, solange `has_reload_map` wahr ist.
    #[prop(into)] on_reload_map: Callback<()>,
    #[prop(into)] has_reload_map: Signal<bool>,
    #[prop(into)] on_change_map_folder: Callback<()>,
) -> impl IntoView {
    let svg_ref: NodeRef<leptos::svg::Svg> = NodeRef::new();
    let drag = StoredValue::new(None::<DragState>);

    let initial = map_image.get_untracked();
    let view_box = RwSignal::new(ViewBox { x: 0.0, y: 0.0, w: initial.width_px, h: initial.height_px });
    let selected_index = RwSignal::new(None::<usize>);
    let add_mode = RwSignal::new(false);
    let visible_kinds = RwSignal::new(FILTER_KINDS.iter().map(|k| k.key.to_string()).collect::<HashSet<String>>());
    let show_exceptions = RwSignal::new(true);

    // Regen-Koordinaten sind bereits deckungsgleich mit dem Minimap-Pixelraum
    // (verifiziert an einer echten regen.txt mit 939 Spawns). `units_per_pixel`
    // bleibt trotzdem vom Backend gesetzt, falls sich das je unterscheidet.
    let upp = move || map_image.with(|m| m.units_per_pixel);

    let raw_count = Memo::new(move |_| lines.with(|l| l.iter().filter(|x| x.kind() == LineKind::Raw).count()));

    let screen_inverse = move || -> Option<Affine> {
        let el = svg_ref.get_untracked()?;
        let g: &web_sys::SvgGraphicsElement = el.unchecked_ref();
        let m = g.get_screen_ctm()?;
        Affine { a: m.a() as f64, b: m.b() as f64, c: m.c() as f64, d: m.d() as f64, e: m.e() as f64, f: m.f() as f64 }
            .inverse()
    };
    let capture = move |pointer_id: i32| {
        if let Some(el) = svg_ref.get_untracked() {
            let el: &web_sys::Element = el.unchecked_ref();
            let _ = el.set_pointer_capture(pointer_id);
        }
    };

    let on_wheel = move |e: web_sys::WheelEvent| {
        e.prevent_default();
        let Some(inverse) = screen_inverse() else { return };
        let (px, py) = inverse.apply(e.client_x() as f64, e.client_y() as f64);
        let width = map_image.with_untracked(|m| m.width_px);
        view_box.update(|vb| *vb = zoom(*vb, px, py, e.delta_y(), width));
    };

    let background_down = move |e: &web_sys::PointerEvent| {
        let Some(inverse) = screen_inverse() else { return };
        let pt = inverse.apply(e.client_x() as f64, e.client_y() as f64);
        if add_mode.get_untracked() {
            let u = upp();
            let mut spawn = new_spawn();
            if let RegenLine::Spawn(s) = &mut spawn {
                s.center_x = js_round(pt.0 * u);
                s.center_y = js_round(pt.1 * u);
            }
            let len = lines.with_untracked(|l| l.len());
            on_add_line.run(spawn);
            add_mode.set(false);
            selected_index.set(Some(len));
            return;
        }
        drag.set_value(Some(DragState::Pan { inverse, start: pt, start_view_box: view_box.get_untracked() }));
        capture(e.pointer_id());
    };

    let marker_down = move |e: &web_sys::PointerEvent, index: usize| {
        if add_mode.get_untracked() {
            return;
        }
        selected_index.set(Some(index));
        let Some(inverse) = screen_inverse() else { return };
        let Some(line) = lines.with_untracked(|l| l.get(index).cloned()) else { return };
        let (center_x, center_y) = match &line {
            RegenLine::Spawn(s) => (s.center_x, s.center_y),
            RegenLine::Exception(x) => (x.center_x, x.center_y),
            RegenLine::Raw { .. } => return,
        };
        drag.set_value(Some(DragState::Move {
            inverse,
            start: inverse.apply(e.client_x() as f64, e.client_y() as f64),
            index,
            original_x: center_x,
            original_y: center_y,
        }));
        capture(e.pointer_id());
    };

    let resize_down = move |e: &web_sys::PointerEvent, index: usize| {
        let Some(inverse) = screen_inverse() else { return };
        let Some(line) = lines.with_untracked(|l| l.get(index).cloned()) else { return };
        let (rx, ry) = match &line {
            RegenLine::Spawn(s) => (s.radius_x, s.radius_y),
            RegenLine::Exception(x) => (x.radius_x, x.radius_y),
            RegenLine::Raw { .. } => return,
        };
        drag.set_value(Some(DragState::Resize {
            inverse,
            start: inverse.apply(e.client_x() as f64, e.client_y() as f64),
            index,
            original_radius_x: rx,
            original_radius_y: ry,
        }));
        capture(e.pointer_id());
    };

    // Ein Handler am <svg>: Marker/Griffe tragen `data-role`/`data-idx`
    // (entspricht den eigenen onPointerDown-Handlern mit stopPropagation).
    let on_pointer_down = move |e: web_sys::PointerEvent| {
        let target = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok());
        let role = target.as_ref().and_then(|t| {
            let role = t.get_attribute("data-role")?;
            let idx = t.get_attribute("data-idx")?.parse::<usize>().ok()?;
            Some((role, idx))
        });
        match role {
            Some((r, idx)) if r == "marker" => marker_down(&e, idx),
            Some((r, idx)) if r == "resize" => resize_down(&e, idx),
            _ => background_down(&e),
        }
    };

    let on_pointer_move = move |e: web_sys::PointerEvent| {
        let Some(d) = drag.get_value() else { return };
        let inv = match d {
            DragState::Pan { inverse, .. } | DragState::Move { inverse, .. } | DragState::Resize { inverse, .. } => inverse,
        };
        let start = match d {
            DragState::Pan { start, .. } | DragState::Move { start, .. } | DragState::Resize { start, .. } => start,
        };
        let current = inv.apply(e.client_x() as f64, e.client_y() as f64);
        let dx = current.0 - start.0;
        let dy = current.1 - start.1;
        let u = upp();
        match d {
            DragState::Pan { start_view_box: s, .. } => {
                view_box.set(ViewBox { x: s.x - dx, y: s.y - dy, w: s.w, h: s.h });
            }
            DragState::Move { index, original_x, original_y, .. } => {
                if let Some(mut l) = lines.with_untracked(|l| l.get(index).cloned()) {
                    match &mut l {
                        RegenLine::Spawn(s) => {
                            s.center_x = js_round(original_x + dx * u);
                            s.center_y = js_round(original_y + dy * u);
                        }
                        RegenLine::Exception(x) => {
                            x.center_x = js_round(original_x + dx * u);
                            x.center_y = js_round(original_y + dy * u);
                        }
                        RegenLine::Raw { .. } => {}
                    }
                    on_update_line.run((index, l));
                }
            }
            DragState::Resize { index, original_radius_x, original_radius_y, .. } => {
                if let Some(mut l) = lines.with_untracked(|l| l.get(index).cloned()) {
                    let rx = 1f64.max(js_round(original_radius_x + dx * u));
                    let ry = 1f64.max(js_round(original_radius_y + dy * u));
                    match &mut l {
                        RegenLine::Spawn(s) => {
                            s.radius_x = rx;
                            s.radius_y = ry;
                        }
                        RegenLine::Exception(x) => {
                            x.radius_x = rx;
                            x.radius_y = ry;
                        }
                        RegenLine::Raw { .. } => {}
                    }
                    on_update_line.run((index, l));
                }
            }
        }
    };

    let toggle_kind = move |key: &'static str| {
        visible_kinds.update(|set| {
            if !set.remove(key) {
                set.insert(key.to_string());
            }
        });
    };

    let line_count = Memo::new(move |_| lines.with(|l| l.len()));
    let stroke_width = move || (view_box.with(|vb| vb.w) / 400.0).max(1.0);

    let marker = move |index: usize| {
        let line = Memo::new(move |_| lines.with(|l| l.get(index).cloned()));
        let visible = Memo::new(move |_| {
            line.with(|l| match l {
                None | Some(RegenLine::Raw { .. }) => false,
                Some(RegenLine::Exception(_)) => show_exceptions.get(),
                Some(RegenLine::Spawn(s)) => visible_kinds.with(|k| k.contains(&first_char(&s.spawn_type))),
            })
        });
        let is_exception = Memo::new(move |_| line.with(|l| matches!(l, Some(RegenLine::Exception(_)))));
        let geom = move || {
            let u = upp();
            line.with(|l| match l {
                Some(RegenLine::Spawn(s)) => (s.center_x / u, s.center_y / u, s.radius_x / u, s.radius_y / u),
                Some(RegenLine::Exception(x)) => (x.center_x / u, x.center_y / u, x.radius_x / u, x.radius_y / u),
                _ => (0.0, 0.0, 0.0, 0.0),
            })
        };
        let color = move || line.with(|l| color_for_spawn_type(l.as_ref().map(|l| l.spawn_type()).unwrap_or("")));
        let selected = move || selected_index.get() == Some(index);
        view! {
            <Show when=move || visible.get()>
                <g>
                    {move || if is_exception.get() {
                        view! {
                            <rect
                                data-role="marker"
                                data-idx=index
                                x=move || { let (cx, _, rx, _) = geom(); cx - rx }
                                y=move || { let (_, cy, _, ry) = geom(); cy - ry }
                                width=move || geom().2 * 2.0
                                height=move || geom().3 * 2.0
                                fill="rgba(248,113,113,0.15)"
                                stroke="#f87171"
                                stroke-width=stroke_width
                                stroke-dasharray=move || { let sw = stroke_width(); format!("{} {}", sw * 3.0, sw * 2.0) }
                            />
                        }
                        .into_any()
                    } else {
                        view! {
                            <ellipse
                                data-role="marker"
                                data-idx=index
                                cx=move || geom().0
                                cy=move || geom().1
                                rx=move || geom().2
                                ry=move || geom().3
                                fill=move || format!("{}33", color())
                                stroke=color
                                stroke-width=move || if selected() { stroke_width() * 2.0 } else { stroke_width() }
                            />
                        }
                        .into_any()
                    }}
                    <circle
                        data-role="marker"
                        data-idx=index
                        cx=move || geom().0
                        cy=move || geom().1
                        r=move || (view_box.with(|vb| vb.w) / 250.0).max(2.0)
                        fill=move || if is_exception.get() { "#f87171" } else { color() }
                    />
                    <Show when=selected>
                        <circle
                            data-role="resize"
                            data-idx=index
                            cx=move || { let (cx, _, rx, _) = geom(); cx + rx }
                            cy=move || { let (_, cy, _, ry) = geom(); cy + ry }
                            r=move || (view_box.with(|vb| vb.w) / 200.0).max(3.0)
                            fill="white"
                            stroke="#111"
                            stroke-width=stroke_width
                            class="cursor-nwse-resize"
                        />
                    </Show>
                </g>
            </Show>
        }
    };

    let selected_line = Memo::new(move |_| selected_index.get().and_then(|i| lines.with(|l| l.get(i).cloned())));

    view! {
        <div class="flex h-full min-h-0 gap-3">
            <div class="relative min-w-0 flex-1 overflow-hidden rounded-lg border border-border bg-black">
                <svg
                    node_ref=svg_ref
                    viewBox=move || view_box.with(|vb| vb.attr())
                    class=move || if add_mode.get() {
                        "h-full w-full touch-none cursor-crosshair"
                    } else {
                        "h-full w-full touch-none cursor-grab"
                    }
                    on:wheel=on_wheel
                    on:pointerdown=on_pointer_down
                    on:pointermove=on_pointer_move
                    on:pointerup=move |_| drag.set_value(None)
                >
                    <image
                        href=move || map_image.with(|m| m.image_data_url.clone())
                        x="0"
                        y="0"
                        width=move || map_image.with(|m| m.width_px)
                        height=move || map_image.with(|m| m.height_px)
                        preserveAspectRatio="none"
                    />
                    <For each=move || 0..line_count.get() key=|i| *i children=move |i| marker(i)/>
                </svg>

                <div class="absolute right-2 top-2 flex gap-1">
                    <Button
                        variant=ButtonVariant::Secondary
                        size=ButtonSize::IconSm
                        on_click=move |_| on_change_map_folder.run(())
                        title="Kartenordner ändern"
                    >
                        <Icon name="FolderCog" class="size-3.5"/>
                    </Button>
                    <Show when=move || has_reload_map.get()>
                        <Button
                            variant=ButtonVariant::Secondary
                            size=ButtonSize::IconSm
                            on_click=move |_| on_reload_map.run(())
                            title="Karte neu laden"
                        >
                            <Icon name="RefreshCw" class="size-3.5"/>
                        </Button>
                    </Show>
                </div>

                <div class="absolute bottom-2 left-2 flex flex-wrap items-center gap-2 rounded-md bg-black/60 px-2 py-1 text-xs text-white">
                    {FILTER_KINDS
                        .iter()
                        .map(|k| {
                            let key = k.key;
                            view! {
                                <label class="flex items-center gap-1">
                                    <input
                                        type="checkbox"
                                        prop:checked=move || visible_kinds.with(|s| s.contains(key))
                                        on:change=move |_| toggle_kind(key)
                                    />
                                    <span style=format!("color: {}", k.color)>{k.label}</span>
                                </label>
                            }
                        })
                        .collect_view()}
                    <label class="flex items-center gap-1">
                        <input
                            type="checkbox"
                            prop:checked=move || show_exceptions.get()
                            on:change=move |_| show_exceptions.update(|v| *v = !*v)
                        />
                        <span class="text-red-400">"Ausnahmezonen"</span>
                    </label>
                    <Show when=move || { raw_count.get() > 0 }>
                        <span class="text-muted-foreground">
                            {move || raw_count.get()} " Kommentarzeile(n) nur in Tabellenansicht"
                        </span>
                    </Show>
                </div>
            </div>

            <div class="flex w-72 shrink-0 flex-col gap-2">
                <button
                    type="button"
                    data-slot="button"
                    class=move || button_class(
                        if add_mode.get() { ButtonVariant::Default } else { ButtonVariant::Outline },
                        ButtonSize::Sm,
                        "",
                    )
                    on:click=move |_| add_mode.update(|v| *v = !*v)
                >
                    <Icon name="Plus" class="size-3.5"/>
                    {move || if add_mode.get() { "Klicke auf die Karte…" } else { "Spawn hinzufügen" }}
                </button>

                <Show
                    when=move || selected_line.with(|l| l.is_some())
                    fallback=|| view! {
                        <p class="text-sm text-muted-foreground">
                            "Klicke auf einen Marker, um ihn zu bearbeiten. Ziehen verschiebt, der Griff unten rechts am Marker skaliert den Radius. Mausrad zoomt, Ziehen auf leerer Fläche verschiebt den Kartenausschnitt."
                        </p>
                    }
                >
                    <div class="space-y-2">
                        <div class="flex items-center justify-between">
                            <span class="text-xs font-medium text-muted-foreground">"Auswahl"</span>
                            <Button variant=ButtonVariant::Ghost size=ButtonSize::IconSm on_click=move |_| selected_index.set(None)>
                                <Icon name="X" class="size-3.5"/>
                            </Button>
                        </div>
                        <RegenLineRow
                            line=Signal::derive(move || {
                                selected_line.get().unwrap_or(RegenLine::Raw { text: String::new() })
                            })
                            on_change=move |l: RegenLine| {
                                if let Some(i) = selected_index.get_untracked() {
                                    on_update_line.run((i, l));
                                }
                            }
                            on_remove=move |_| {
                                if let Some(i) = selected_index.get_untracked() {
                                    on_remove_line.run(i);
                                    selected_index.set(None);
                                }
                            }
                        />
                    </div>
                </Show>
            </div>
        </div>
    }
}
