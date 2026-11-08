//! Reiner Rust-Ersatz für den früheren CodeMirror-Editor: ein `<textarea>`
//! (Eingabe, Auswahl, Rückgängig über den Browser) mit einer synchron
//! mitscrollenden, hervorgehobenen Overlay-Ebene (siehe [`lua_highlight`]),
//! Zeilennummern und Hervorhebung der aktiven Zeile.
//!
//! Übernommene CodeMirror-Funktionen, die der Quest Builder nutzte:
//! Lua-Hervorhebung, Zeilennummern, aktive Zeile, Hell/Dunkel-Thema,
//! `onChange`, Einfügen an der Cursor-Position (Funktionsliste) und
//! "zu Zeile springen + markieren" (Volltextsuche). Tab rückt ein.
//!
//! [`lua_highlight`]: super::lua_highlight

use super::lua_highlight::highlight_html;
use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// Zeilenhöhe in px - muss für Textarea, Overlay und Zeilennummern gleich sein.
const LINE_HEIGHT: f64 = 18.0;
/// Innenabstand oben/unten (px), ebenfalls überall gleich.
const PAD_Y: f64 = 4.0;
const TEXT_STYLE: &str = "line-height:18px;tab-size:4;-moz-tab-size:4;padding:4px 8px;margin:0;font-variant-ligatures:none;";

/// Steuert einen gemounteten [`CodeEditor`] von außen (wie früher die
/// `ReactCodeMirrorRef`). Ist kein Editor gemountet, tun die Methoden nichts.
#[derive(Clone, Copy)]
pub struct CodeEditorHandle {
    textarea: NodeRef<html::Textarea>,
}

impl Default for CodeEditorHandle {
    fn default() -> Self {
        Self::new()
    }
}

/// UTF-16-Offset (wie `selectionStart`) → Byte-Offset in `text`.
fn utf16_to_byte(text: &str, pos: u32) -> usize {
    let mut n = 0u32;
    for (i, c) in text.char_indices() {
        if n >= pos {
            return i;
        }
        n += c.len_utf16() as u32;
    }
    text.len()
}

fn byte_to_utf16(text: &str, byte: usize) -> u32 {
    text[..byte].encode_utf16().count() as u32
}

/// 0-basierte Zeile, in der `byte` liegt.
fn line_of(text: &str, byte: usize) -> usize {
    text[..byte.min(text.len())].matches('\n').count()
}

impl CodeEditorHandle {
    pub fn new() -> Self {
        Self { textarea: NodeRef::new() }
    }

    pub fn is_mounted(&self) -> bool {
        self.textarea.get_untracked().is_some()
    }

    /// Wie [`is_mounted`](Self::is_mounted), aber reaktiv (für Effekte).
    pub fn track_mounted(&self) -> bool {
        self.textarea.get().is_some()
    }

    /// Ersetzt die aktuelle Auswahl durch `text`, setzt den Cursor dahinter
    /// und fokussiert den Editor. Löst wie eine Eingabe `on_change` aus.
    pub fn insert_at_cursor(&self, text: &str) {
        let Some(ta) = self.textarea.get_untracked() else { return };
        let _ = ta.focus();
        // execCommand hält den Browser-Rückgängig-Verlauf intakt und feuert
        // ein normales `input`-Event.
        let done = ta
            .owner_document()
            .and_then(|d| d.dyn_into::<web_sys::HtmlDocument>().ok())
            .and_then(|d| d.exec_command_with_show_ui_and_value("insertText", false, text).ok())
            .unwrap_or(false);
        if !done {
            let start = ta.selection_start().ok().flatten().unwrap_or(0);
            let end = ta.selection_end().ok().flatten().unwrap_or(start);
            let _ = ta.set_range_text_with_start_and_end_and_mode(text, start, end, "end");
            if let Ok(ev) = web_sys::Event::new_with_event_init_dict("input", &{
                let init = web_sys::EventInit::new();
                init.set_bubbles(true);
                init
            }) {
                let _ = ta.dispatch_event(&ev);
            }
        }
    }

    /// Markiert Zeile `line` (1-basiert, wird auf den Text begrenzt) von
    /// `text` (dem aktuellen Inhalt), scrollt sie in den sichtbaren Bereich
    /// und fokussiert den Editor.
    pub fn select_line(&self, line: usize, text: &str) {
        let Some(ta) = self.textarea.get_untracked() else { return };
        if ta.value() != text {
            ta.set_value(text);
        }
        let lines: Vec<&str> = text.split('\n').collect();
        let idx = line.clamp(1, lines.len()) - 1;
        let from: usize = lines[..idx].iter().map(|l| l.len() + 1).sum();
        let to = from + lines[idx].len();
        let _ = ta.focus();
        let _ = ta.set_selection_range(byte_to_utf16(text, from), byte_to_utf16(text, to));
        let line_top = PAD_Y + idx as f64 * LINE_HEIGHT;
        let view_h = ta.client_height() as f64;
        let top = ta.scroll_top() as f64;
        if line_top < top || line_top + LINE_HEIGHT > top + view_h {
            ta.set_scroll_top((line_top - view_h / 2.0).max(0.0) as i32);
        }
        // Scroll-Ereignis kommt asynchron - Overlay sofort nachziehen.
        let _ = ta.dispatch_event(&web_sys::Event::new("scroll").unwrap());
        let _ = ta.dispatch_event(&web_sys::Event::new("select").unwrap());
    }
}

#[component]
pub fn CodeEditor(
    #[prop(into)] value: Signal<String>,
    #[prop(into)] on_change: Callback<String>,
    #[prop(optional)] handle: Option<CodeEditorHandle>,
    #[prop(into, optional)] class: MaybeProp<String>,
) -> impl IntoView {
    let handle = handle.unwrap_or_default();
    let textarea = handle.textarea;
    let scroll = RwSignal::new((0.0f64, 0.0f64));
    let active_line = RwSignal::new(0usize);
    let focused = RwSignal::new(false);

    let update_active = move || {
        if let Some(ta) = textarea.get_untracked() {
            let text = ta.value();
            let pos = ta.selection_start().ok().flatten().unwrap_or(0);
            active_line.set(line_of(&text, utf16_to_byte(&text, pos)));
        }
    };
    let sync_scroll = move || {
        if let Some(ta) = textarea.get_untracked() {
            scroll.set((ta.scroll_top() as f64, ta.scroll_left() as f64));
        }
    };

    let line_numbers = Memo::new(move |_| {
        let n = value.with(|v| v.matches('\n').count() + 1);
        (1..=n).map(|i| i.to_string()).collect::<Vec<_>>().join("\n")
    });
    // Zeilenumbruch am Ende würde vom <pre> verschluckt - eine Leerzeile
    // anhängen, damit Overlay und Textarea gleich hoch sind.
    let highlighted = Memo::new(move |_| value.with(|v| format!("{}\n ", highlight_html(v))));

    let on_keydown = move |e: web_sys::KeyboardEvent| {
        if e.key() == "Tab" && !e.shift_key() && !e.ctrl_key() && !e.alt_key() && !e.meta_key() {
            e.prevent_default();
            handle.insert_at_cursor("\t");
        }
    };

    view! {
        <div class=move || format!("relative flex h-full overflow-hidden bg-background font-mono text-xs {}", class.get().unwrap_or_default())>
            // Zeilennummern
            <div class="relative shrink-0 overflow-hidden border-r border-border bg-muted/30 text-right text-muted-foreground select-none">
                <pre
                    aria-hidden="true"
                    class="font-mono"
                    style=move || format!("{TEXT_STYLE}transform:translateY({}px);", -scroll.get().0)
                >
                    {move || line_numbers.get()}
                </pre>
            </div>
            <div class="relative min-w-0 flex-1 overflow-hidden">
                // aktive Zeile
                <div
                    aria-hidden="true"
                    class=move || if focused.get() { "pointer-events-none absolute inset-x-0 bg-muted/60" } else { "pointer-events-none absolute inset-x-0 bg-muted/30" }
                    style=move || format!("height:{LINE_HEIGHT}px;top:{}px;", PAD_Y + active_line.get() as f64 * LINE_HEIGHT - scroll.get().0)
                ></div>
                // hervorgehobener Text
                <pre
                    aria-hidden="true"
                    class="pointer-events-none absolute left-0 top-0 min-w-full whitespace-pre font-mono text-foreground"
                    style=move || {
                        let (t, l) = scroll.get();
                        format!("{TEXT_STYLE}transform:translate({}px,{}px);", -l, -t)
                    }
                    inner_html=move || highlighted.get()
                ></pre>
                // Eingabe (Text unsichtbar, nur Cursor und Auswahl sichtbar)
                <textarea
                    node_ref=textarea
                    wrap="off"
                    spellcheck="false"
                    autocomplete="off"
                    autocapitalize="off"
                    class="absolute inset-0 h-full w-full resize-none overflow-auto whitespace-pre border-0 bg-transparent font-mono text-transparent caret-foreground outline-none selection:bg-primary/25"
                    style=TEXT_STYLE
                    prop:value=move || value.get()
                    on:input=move |e| {
                        on_change.run(event_target_value(&e));
                        update_active();
                    }
                    on:scroll=move |_| sync_scroll()
                    on:keydown=on_keydown
                    on:keyup=move |_| update_active()
                    on:click=move |_| update_active()
                    on:select=move |_| update_active()
                    on:focus=move |_| {
                        focused.set(true);
                        update_active();
                    }
                    on:blur=move |_| focused.set(false)
                ></textarea>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets() {
        let t = "ä\n😀x";
        assert_eq!(utf16_to_byte(t, 0), 0);
        assert_eq!(utf16_to_byte(t, 1), 2);
        assert_eq!(utf16_to_byte(t, 2), 3);
        assert_eq!(utf16_to_byte(t, 4), 7);
        assert_eq!(byte_to_utf16(t, 7), 4);
        assert_eq!(line_of(t, 3), 1);
    }
}
