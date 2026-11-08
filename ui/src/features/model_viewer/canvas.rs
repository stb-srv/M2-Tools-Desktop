//! `Gr2Canvas`: wiederverwendbare 3D-Ansicht eines Gr2-Modells (Model
//! Viewer, NPC-Vorschau im Shop Editor). Reines WebGL2 aus Rust.

use super::renderer::Renderer;
use super::types::Gr2ModelInfo;
use leptos::prelude::*;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, HtmlElement, ResizeObserver};

#[derive(Clone, Copy)]
enum DragMode {
    Rotate,
    Pan,
    Dolly,
}

struct Viewer {
    renderer: Renderer,
    container: HtmlElement,
    canvas: HtmlCanvasElement,
    raf_id: Option<i32>,
    raf_cb: Option<Closure<dyn FnMut()>>,
    observer: Option<ResizeObserver>,
    _resize_cb: Option<Closure<dyn FnMut(js_sys::Array)>>,
    drag: Option<(DragMode, i32, f64, f64)>,
}

type ViewerSlot = StoredValue<Option<Viewer>, LocalStorage>;

impl Viewer {
    fn start(container: HtmlElement, canvas: HtmlCanvasElement, slot: ViewerSlot) -> Result<Self, String> {
        let mut renderer = Renderer::new(canvas.clone())?;
        renderer.set_size(container.client_width().max(0) as u32, container.client_height().max(0) as u32);

        let raf_cb = Closure::<dyn FnMut()>::new(move || {
            slot.try_update_value(|v| {
                if let Some(v) = v {
                    v.tick();
                }
            });
        });

        let c2 = container.clone();
        let resize_cb = Closure::<dyn FnMut(js_sys::Array)>::new(move |_entries: js_sys::Array| {
            let (w, h) = (c2.client_width(), c2.client_height());
            if w <= 0 || h <= 0 {
                return;
            }
            slot.try_update_value(|v| {
                if let Some(v) = v {
                    v.renderer.set_size(w as u32, h as u32);
                }
            });
        });
        let observer = ResizeObserver::new(resize_cb.as_ref().unchecked_ref()).ok();
        if let Some(o) = &observer {
            o.observe(&container);
        }

        let raf_id = web_sys::window().and_then(|w| w.request_animation_frame(raf_cb.as_ref().unchecked_ref()).ok());

        Ok(Self {
            renderer,
            container,
            canvas,
            raf_id,
            raf_cb: Some(raf_cb),
            observer,
            _resize_cb: Some(resize_cb),
            drag: None,
        })
    }

    fn tick(&mut self) {
        // Bereiche bleiben beim Wegnavigieren versteckt gemountet - solange
        // der Container `display:none` ist (offsetParent == null), nichts
        // rendern, damit ein Hintergrund-Tab keine GPU-Zeit verbrennt.
        if self.container.offset_parent().is_some() {
            self.renderer.render();
        }
        if let (Some(w), Some(cb)) = (web_sys::window(), self.raf_cb.as_ref()) {
            self.raf_id = w.request_animation_frame(cb.as_ref().unchecked_ref()).ok();
        }
    }

    fn client_height(&self) -> f32 {
        self.canvas.client_height() as f32
    }
}

impl Drop for Viewer {
    fn drop(&mut self) {
        if let (Some(w), Some(id)) = (web_sys::window(), self.raf_id) {
            let _ = w.cancel_animation_frame(id);
        }
        if let Some(o) = &self.observer {
            o.disconnect();
        }
    }
}

#[component]
pub fn Gr2Canvas(
    #[prop(into)] model: Signal<Option<Arc<Gr2ModelInfo>>>,
    #[prop(into, optional)] class: MaybeProp<String>,
) -> impl IntoView {
    let container = NodeRef::<leptos::html::Div>::new();
    let canvas = NodeRef::<leptos::html::Canvas>::new();
    let slot: ViewerSlot = StoredValue::new_local(None);

    Effect::new(move |_| {
        let m = model.get();
        let (Some(div), Some(cv)) = (container.get(), canvas.get()) else { return };
        if slot.with_value(|v| v.is_none()) {
            let div: HtmlElement = div.into();
            let cv: HtmlCanvasElement = cv;
            match Viewer::start(div, cv, slot) {
                Ok(v) => slot.set_value(Some(v)),
                Err(e) => {
                    web_sys::console::error_1(&format!("3D-Ansicht: {e}").into());
                    return;
                }
            }
        }
        slot.update_value(|v| {
            if let Some(v) = v {
                v.renderer.set_model(m.as_deref(), move |generation, idx, img| {
                    slot.try_update_value(|v| {
                        if let Some(v) = v {
                            v.renderer.upload_texture(generation, idx, &img);
                        }
                    });
                });
            }
        });
    });

    on_cleanup(move || {
        slot.try_update_value(|v| {
            v.take();
        });
    });

    let on_down = move |e: web_sys::PointerEvent| {
        let mode = match e.button() {
            0 if e.ctrl_key() || e.meta_key() || e.shift_key() => DragMode::Pan,
            0 => DragMode::Rotate,
            1 => DragMode::Dolly,
            2 => DragMode::Pan,
            _ => return,
        };
        if let Some(cv) = canvas.get_untracked() {
            let _ = cv.set_pointer_capture(e.pointer_id());
        }
        slot.try_update_value(|v| {
            if let Some(v) = v {
                v.drag = Some((mode, e.pointer_id(), e.client_x() as f64, e.client_y() as f64));
            }
        });
    };
    let on_move = move |e: web_sys::PointerEvent| {
        slot.try_update_value(|v| {
            let Some(v) = v else { return };
            let Some((mode, id, x, y)) = v.drag else { return };
            if id != e.pointer_id() {
                return;
            }
            let (nx, ny) = (e.client_x() as f64, e.client_y() as f64);
            let (dx, dy) = ((nx - x) as f32, (ny - y) as f32);
            let h = v.client_height();
            match mode {
                DragMode::Rotate => v.renderer.orbit.rotate(dx, dy, h),
                DragMode::Pan => v.renderer.orbit.pan(dx, dy, h),
                DragMode::Dolly => v.renderer.orbit.dolly(dy),
            }
            v.drag = Some((mode, id, nx, ny));
        });
    };
    let on_up = move |e: web_sys::PointerEvent| {
        if let Some(cv) = canvas.get_untracked() {
            let _ = cv.release_pointer_capture(e.pointer_id());
        }
        slot.try_update_value(|v| {
            if let Some(v) = v {
                v.drag = None;
            }
        });
    };

    view! {
        <div node_ref=container class=move || class.get().unwrap_or_default()>
            <canvas
                node_ref=canvas
                style="display:block;touch-action:none"
                on:pointerdown=on_down
                on:pointermove=on_move
                on:pointerup=on_up
                on:pointercancel=on_up
                on:contextmenu=|e| e.prevent_default()
                on:wheel=move |e: web_sys::WheelEvent| {
                    e.prevent_default();
                    slot.try_update_value(|v| {
                        if let Some(v) = v {
                            v.renderer.orbit.wheel(e.delta_y() as f32, e.delta_mode(), e.ctrl_key());
                        }
                    });
                }
            ></canvas>
        </div>
    }
}
