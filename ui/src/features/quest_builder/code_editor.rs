//! Reiner Rust-Ersatz für den früheren CodeMirror-Editor: ein `<textarea>`
//! (Eingabe und Auswahl) mit einer synchron mitscrollenden, hervorgehobenen
//! Overlay-Ebene (siehe [`lua_highlight`]), Zeilennummern und Hervorhebung
//! der aktiven Zeile.
//!
//! Funktionen wie im CodeMirror-`basicSetup`, das der Quest Builder nutzte:
//! Lua-Hervorhebung, Zeilennummern, aktive Zeile, Hell/Dunkel-Thema,
//! Code-Falten (Pfeile in der Leiste, Strg+Umschalt+[ / ], Strg+Alt+[ / ]),
//! Suchen und Ersetzen (Strg+F, F3), Autovervollständigung (beim Tippen,
//! Strg+Leertaste), eigenes Rückgängig/Wiederholen (Strg+Z / Strg+Y),
//! `onChange`, Einfügen an der Cursor-Position (Funktionsliste) und "zu
//! Zeile springen + markieren" (Volltextsuche). Tab rückt ein.
//!
//! Gefaltete Zeilen stehen nicht in der Textarea; [`Doc`] rechnet zwischen
//! Ansicht und vollem Text um.
//!
//! [`lua_highlight`]: super::lua_highlight

use super::editor_complete::{complete, doc_words, word_start, Completion};
use super::editor_doc::{fold_ranges, line_of, line_starts, Doc};
use super::editor_search::{find_all, next_match, replacement_for, SearchOpts, MAX_MATCHES};
use super::lua_highlight::{escape_html, highlight_lines};
use crate::components::icons::Icon;
use leptos::html;
use leptos::prelude::*;
use std::time::Duration;
use wasm_bindgen::JsCast;

/// Zeilenhöhe in px - muss für Textarea, Overlay und Zeilennummern gleich sein.
const LINE_HEIGHT: f64 = 18.0;
/// Innenabstand oben/unten (px), ebenfalls überall gleich.
const PAD_Y: f64 = 4.0;
/// Innenabstand links (px).
const PAD_X: f64 = 8.0;
const TAB_SIZE: usize = 4;
const TEXT_STYLE: &str = "line-height:18px;tab-size:4;-moz-tab-size:4;padding:4px 8px;margin:0;font-variant-ligatures:none;";
/// Platzhalter hinter einer gefalteten Kopfzeile.
const FOLD_PLACEHOLDER: &str =
    "<span class=\"ml-1 rounded-sm border border-border bg-muted px-1 text-muted-foreground\">⋯</span>";
/// So viele Schritte merkt sich Rückgängig.
const MAX_UNDO: usize = 300;
/// Gleichartige Eingaben innerhalb dieser Zeit werden zu einem
/// Rückgängig-Schritt zusammengefasst (wie CodeMirrors `newGroupDelay`).
const UNDO_MERGE_MS: f64 = 500.0;

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
    text[..byte.min(text.len())].encode_utf16().count() as u32
}

/// Anzeigespalte (Tabs auf 4 erweitert) von `s`.
fn display_cols(s: &str) -> usize {
    s.chars().fold(0, |col, c| if c == '\t' { (col / TAB_SIZE + 1) * TAB_SIZE } else { col + 1 })
}

#[derive(Clone, Debug, Default)]
struct Snap {
    text: String,
    folds: Vec<(usize, usize)>,
    /// Auswahl im vollen Text (Bytes).
    sel: (usize, usize),
}

#[derive(Default)]
struct Core {
    doc: Doc,
    /// Inhalt der Textarea (= `doc.view()`).
    view: String,
    undo: Vec<Snap>,
    redo: Vec<Snap>,
    /// Art und Zeitpunkt der letzten Eingabe (zum Zusammenfassen).
    last_kind: Option<(String, f64)>,
    /// Auswahl (UTF-16) vor der laufenden Eingabe.
    sel_before: Option<(u32, u32)>,
    on_change: Option<Callback<String>>,
    /// Eingabe stammt aus der Autovervollständigung (nicht erneut öffnen).
    accepting: bool,
}

/// Steuert einen gemounteten [`CodeEditor`] von außen (wie früher die
/// `ReactCodeMirrorRef`). Ist kein Editor gemountet, tun die Methoden nichts.
#[derive(Clone, Copy)]
pub struct CodeEditorHandle {
    textarea: NodeRef<html::Textarea>,
    core: StoredValue<Core>,
    /// Wird bei jeder Änderung von Text oder Faltung erhöht.
    rev: RwSignal<u64>,
}

impl Default for CodeEditorHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeEditorHandle {
    pub fn new() -> Self {
        Self { textarea: NodeRef::new(), core: StoredValue::new(Core::default()), rev: RwSignal::new(0) }
    }

    pub fn is_mounted(&self) -> bool {
        self.textarea.get_untracked().is_some()
    }

    /// Wie [`is_mounted`](Self::is_mounted), aber reaktiv (für Effekte).
    pub fn track_mounted(&self) -> bool {
        self.textarea.get().is_some()
    }

    /// Auswahl der Textarea als (Start, Ende) in UTF-16.
    fn sel16(&self) -> Option<(u32, u32)> {
        let ta = self.textarea.get_untracked()?;
        let s = ta.selection_start().ok().flatten().unwrap_or(0);
        let e = ta.selection_end().ok().flatten().unwrap_or(s);
        Some((s, e))
    }

    /// Aktuelle Auswahl im vollen Text (Bytes).
    fn sel_full(&self) -> (usize, usize) {
        let Some((s, e)) = self.sel16() else { return (0, 0) };
        self.core.with_value(|c| {
            let vs = utf16_to_byte(&c.view, s);
            let ve = utf16_to_byte(&c.view, e);
            (c.doc.to_full(vs, false), c.doc.to_full(ve, false))
        })
    }

    /// Bringt die Textarea auf den Stand von `doc` und setzt optional die
    /// Auswahl (voller Text). Gibt die neue Ansicht zurück.
    fn sync(&self, sel: Option<(usize, usize)>) {
        let (view, sel16) = self.core.try_update_value(|c| {
            c.view = c.doc.view();
            let sel16 = sel.map(|(s, e)| {
                let (vs, ve) = (c.doc.to_view(s), c.doc.to_view(e));
                (byte_to_utf16(&c.view, vs), byte_to_utf16(&c.view, ve))
            });
            (c.view.clone(), sel16)
        })
        .unwrap_or_default();
        if let Some(ta) = self.textarea.get_untracked() {
            if ta.value() != view {
                ta.set_value(&view);
            }
            if let Some((s, e)) = sel16 {
                let _ = ta.set_selection_range(s, e);
                self.scroll_to_caret(&ta, &view, false);
                let _ = ta.dispatch_event(&web_sys::Event::new("select").unwrap());
            }
        }
        self.rev.update(|r| *r += 1);
    }

    /// Scrollt die Zeile des Auswahlbeginns in den sichtbaren Bereich
    /// (`center`: mittig, sonst nur falls nötig).
    fn scroll_to_caret(&self, ta: &web_sys::HtmlTextAreaElement, view: &str, center: bool) {
        let s = ta.selection_start().ok().flatten().unwrap_or(0);
        let line = line_of(view, utf16_to_byte(view, s));
        let line_top = PAD_Y + line as f64 * LINE_HEIGHT;
        let view_h = ta.client_height() as f64;
        let top = ta.scroll_top() as f64;
        if center || line_top < top || line_top + LINE_HEIGHT > top + view_h {
            ta.set_scroll_top((line_top - view_h / 2.0).max(0.0) as i32);
        }
        // Scroll-Ereignis kommt asynchron - Overlay sofort nachziehen.
        let _ = ta.dispatch_event(&web_sys::Event::new("scroll").unwrap());
    }

    fn emit_change(&self) {
        let (cb, text) = self.core.with_value(|c| (c.on_change, c.doc.text.clone()));
        if let Some(cb) = cb {
            cb.run(text);
        }
    }

    fn snapshot(c: &Core, sel: (usize, usize)) -> Snap {
        Snap { text: c.doc.text.clone(), folds: c.doc.folds.clone(), sel }
    }

    fn push_undo(c: &mut Core, snap: Snap) {
        c.undo.push(snap);
        if c.undo.len() > MAX_UNDO {
            c.undo.remove(0);
        }
        c.redo.clear();
    }

    /// Ersetzt `start..end` (voller Text) durch `ins` als eigener
    /// Rückgängig-Schritt und wählt danach `sel` aus.
    fn edit_full(&self, start: usize, end: usize, ins: &str, sel: Option<(usize, usize)>) {
        let before = self.sel_full();
        self.core.update_value(|c| {
            let snap = Self::snapshot(c, before);
            Self::push_undo(c, snap);
            c.last_kind = None;
            c.doc.apply(start, end, ins);
        });
        self.sync(sel);
        self.emit_change();
    }

    fn undo_redo(&self, redo: bool) {
        let cur = self.sel_full();
        let sel = self.core.try_update_value(|c| {
            let snap = if redo { c.redo.pop() } else { c.undo.pop() }?;
            let now = Self::snapshot(c, cur);
            if redo {
                c.undo.push(now);
            } else {
                c.redo.push(now);
            }
            c.doc.text = snap.text;
            c.doc.folds = snap.folds;
            c.last_kind = None;
            Some(snap.sel)
        });
        if let Some(Some(sel)) = sel {
            self.sync(Some(sel));
            self.emit_change();
        }
    }

    /// Ersetzt die aktuelle Auswahl durch `text`, setzt den Cursor dahinter
    /// und fokussiert den Editor. Löst wie eine Eingabe `on_change` aus.
    pub fn insert_at_cursor(&self, text: &str) {
        let Some(ta) = self.textarea.get_untracked() else { return };
        let _ = ta.focus();
        let sel = self.sel16();
        self.core.update_value(|c| c.sel_before = sel);
        // execCommand feuert ein normales `input`-Event (-> Rückgängig,
        // Faltungen, on_change laufen über denselben Weg wie Tippen).
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
    /// `text` (dem aktuellen Inhalt), klappt sie falls nötig auf, scrollt
    /// sie in den sichtbaren Bereich und fokussiert den Editor.
    pub fn select_line(&self, line: usize, text: &str) {
        let Some(ta) = self.textarea.get_untracked() else { return };
        self.core.update_value(|c| {
            if c.doc.text != text {
                c.doc = Doc::new(text.to_string());
                c.undo.clear();
                c.redo.clear();
            }
        });
        let (from, to) = self.core.try_update_value(|c| {
            let ls = line_starts(&c.doc.text);
            let idx = line.clamp(1, ls.len()) - 1;
            let from = ls[idx];
            let to = ls.get(idx + 1).map(|n| n - 1).unwrap_or(c.doc.text.len());
            c.doc.reveal(from, to.max(from + 1).min(c.doc.text.len().max(from)));
            (from, to)
        })
        .unwrap_or_default();
        let _ = ta.focus();
        self.sync(Some((from, to)));
        let view = self.core.with_value(|c| c.view.clone());
        self.scroll_to_caret(&ta, &view, false);
    }

    /// Klappt den Block in Zeile `line` (voller Text) auf bzw. zu.
    fn toggle_fold(&self, line: usize) {
        let sel = self.sel_full();
        let sel = self.core.try_update_value(|c| {
            let ranges = fold_ranges(&c.doc.text);
            c.doc.toggle_line(line, &ranges);
            // Cursor in nun verborgenen Zeilen ans Ende der Kopfzeile.
            let fix = |x: usize| c.doc.to_full(c.doc.to_view(x), false);
            (fix(sel.0), fix(sel.1))
        });
        self.sync(sel);
    }

    /// Faltet (`fold`) bzw. entfaltet den Block um den Cursor.
    fn fold_at_cursor(&self, fold: bool) {
        let (s, _) = self.sel_full();
        let sel = self.core.try_update_value(|c| {
            let line = line_of(&c.doc.text, s);
            let ranges = fold_ranges(&c.doc.text);
            if fold {
                // Innersten noch offenen Block falten.
                let mut l = line;
                loop {
                    let Some(h) = Doc::enclosing_block(l, &ranges) else { break };
                    if !c.doc.is_folded_line(h) {
                        c.doc.fold_line(h, &ranges);
                        break;
                    }
                    if h == 0 {
                        break;
                    }
                    l = h - 1;
                }
            } else if !c.doc.unfold_line(line) {
                if let Some(h) = Doc::enclosing_block(line, &ranges) {
                    c.doc.unfold_line(h);
                }
            }
            let fix = |x: usize| c.doc.to_full(c.doc.to_view(x), false);
            (fix(s), fix(s))
        });
        self.sync(sel);
    }

    fn fold_all(&self, fold: bool) {
        let (s, _) = self.sel_full();
        let sel = self.core.try_update_value(|c| {
            if fold {
                let ranges = fold_ranges(&c.doc.text);
                c.doc.fold_all(&ranges);
            } else {
                c.doc.unfold_all();
            }
            let x = c.doc.to_full(c.doc.to_view(s), false);
            (x, x)
        });
        self.sync(sel);
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct SearchUi {
    open: bool,
    query: String,
    replace: String,
    opts: SearchOpts,
    /// Index des aktuell ausgewählten Treffers.
    current: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
struct CompUi {
    items: Vec<Completion>,
    sel: usize,
    /// Wortbeginn in der Ansicht (Bytes).
    start: usize,
    /// Position unter dem Cursor (ohne Scroll), px.
    top: f64,
    left: f64,
}

#[component]
pub fn CodeEditor(
    #[prop(into)] value: Signal<String>,
    #[prop(into)] on_change: Callback<String>,
    #[prop(optional)] handle: Option<CodeEditorHandle>,
    #[prop(into, optional)] class: MaybeProp<String>,
) -> impl IntoView {
    let h = handle.unwrap_or_default();
    h.core.update_value(|c| c.on_change = Some(on_change));
    let textarea = h.textarea;
    let rev = h.rev;
    let scroll = RwSignal::new((0.0f64, 0.0f64));
    let active_line = RwSignal::new(0usize);
    let focused = RwSignal::new(false);
    let search = RwSignal::new(SearchUi::default());
    let comp = RwSignal::new(None::<CompUi>);
    let find_input = NodeRef::<html::Input>::new();
    let measure = NodeRef::<html::Span>::new();
    let comp_list = NodeRef::<html::Ul>::new();

    // Neuer Inhalt von außen (Datei geladen, Vorlage) -> Dokument ersetzen.
    Effect::new(move |_| {
        let v = value.get();
        let mounted = textarea.get().is_some();
        let changed = h
            .core
            .try_update_value(|c| {
                if c.doc.text != v {
                    c.doc = Doc::new(v);
                    c.undo.clear();
                    c.redo.clear();
                    c.last_kind = None;
                    true
                } else {
                    false
                }
            })
            .unwrap_or(false);
        if changed {
            comp.set(None);
        }
        if mounted || changed {
            h.sync(None);
        }
    });

    let text_m = Memo::new(move |_| {
        rev.track();
        h.core.with_value(|c| c.doc.text.clone())
    });
    let lines_html = Memo::new(move |_| text_m.with(|t| highlight_lines(t)));
    let ranges = Memo::new(move |_| text_m.with(|t| fold_ranges(t)));
    // (sichtbare Zeilen, gefaltete Kopfzeilen)
    let layout = Memo::new(move |_| {
        rev.track();
        h.core.with_value(|c| {
            let folded: Vec<usize> = c.doc.folds.iter().map(|&(a, _)| line_of(&c.doc.text, a)).collect();
            (c.doc.visible_lines(), folded)
        })
    });

    let matches = Memo::new(move |_| {
        let (open, q, o) = search.with(|s| (s.open, s.query.clone(), s.opts));
        if !open {
            return Ok(Vec::new());
        }
        text_m.with(|t| find_all(t, &q, o))
    });

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

    // ---- Autovervollständigung ----
    let char_width = move || {
        measure.get_untracked().map(|m| m.get_bounding_client_rect().width() / 10.0).filter(|w| *w > 0.0).unwrap_or(7.2)
    };
    let open_completion = move |explicit: bool| {
        let Some((s, e)) = h.sel16() else { return };
        if s != e {
            comp.set(None);
            return;
        }
        let cw = char_width();
        let next = h.core.with_value(|c| {
            let caret = utf16_to_byte(&c.view, s);
            let start = word_start(&c.view, caret);
            let prefix = &c.view[start..caret];
            if (!explicit && prefix.is_empty()) || prefix.starts_with(|ch: char| ch.is_ascii_digit()) {
                return None;
            }
            let full = (c.doc.to_full(start, false), c.doc.to_full(caret, false));
            let items = complete(prefix, &doc_words(&c.doc.text, full));
            if items.is_empty() {
                return None;
            }
            let line = line_of(&c.view, start);
            let line_start = c.view[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let col = display_cols(&c.view[line_start..start]);
            Some(CompUi {
                items,
                sel: 0,
                start,
                top: PAD_Y + (line + 1) as f64 * LINE_HEIGHT,
                left: PAD_X + col as f64 * cw,
            })
        });
        comp.set(next);
    };
    let accept_completion = move |i: usize| {
        let Some(ui) = comp.get_untracked() else { return };
        let Some(item) = ui.items.get(i) else { return };
        let Some(ta) = textarea.get_untracked() else { return };
        let Some((_, caret)) = h.sel16() else { return };
        let start16 = h.core.with_value(|c| byte_to_utf16(&c.view, ui.start));
        comp.set(None);
        let _ = ta.set_selection_range(start16, caret);
        h.core.update_value(|c| c.accepting = true);
        h.insert_at_cursor(&item.label);
        h.core.update_value(|c| c.accepting = false);
    };
    let move_completion = move |delta: isize| {
        comp.update(|c| {
            if let Some(c) = c {
                let n = c.items.len() as isize;
                c.sel = ((c.sel as isize + delta).rem_euclid(n)) as usize;
            }
        });
        if let (Some(ul), Some(ui)) = (comp_list.get_untracked(), comp.get_untracked()) {
            if let Some(li) = ul.children().item(ui.sel as u32) {
                li.scroll_into_view_with_bool(false);
            }
        }
    };

    // ---- Suche ----
    let select_match = move |i: usize| {
        let Ok(ms) = matches.get_untracked() else { return };
        let Some(&(a, b)) = ms.get(i) else { return };
        h.core.update_value(|c| {
            c.doc.reveal(a, b);
        });
        h.sync(Some((a, b)));
        if let Some(ta) = textarea.get_untracked() {
            let view = h.core.with_value(|c| c.view.clone());
            h.scroll_to_caret(&ta, &view, false);
        }
        search.update(|s| s.current = Some(i));
    };
    let find_next = move |back: bool| {
        let Ok(ms) = matches.get_untracked() else { return };
        let (s, e) = h.sel_full();
        if let Some(i) = next_match(&ms, if back { s } else { e }, back) {
            select_match(i);
        }
    };
    let open_search = move || {
        let (s, e) = h.sel_full();
        let selected = h.core.with_value(|c| c.doc.text.get(s..e).map(str::to_string)).unwrap_or_default();
        search.update(|st| {
            st.open = true;
            if !selected.is_empty() && !selected.contains('\n') {
                st.query = selected;
            }
            st.current = None;
        });
        set_timeout(
            move || {
                if let Some(inp) = find_input.get_untracked() {
                    let _ = inp.focus();
                    inp.select();
                }
            },
            Duration::ZERO,
        );
    };
    let close_search = move || {
        search.update(|s| {
            s.open = false;
            s.current = None;
        });
        if let Some(ta) = textarea.get_untracked() {
            let _ = ta.focus();
        }
    };
    let replace_one = move || {
        let Ok(ms) = matches.get_untracked() else { return };
        let sel = h.sel_full();
        if !ms.contains(&sel) {
            find_next(false);
            return;
        }
        let (q, r, o) = search.with_untracked(|s| (s.query.clone(), s.replace.clone(), s.opts));
        let matched = h.core.with_value(|c| c.doc.text[sel.0..sel.1].to_string());
        let rep = replacement_for(&matched, &q, &r, o);
        let after = sel.0 + rep.len();
        h.edit_full(sel.0, sel.1, &rep, Some((after, after)));
        find_next(false);
    };
    let replace_all = move || {
        let Ok(ms) = matches.get_untracked() else { return };
        if ms.is_empty() {
            return;
        }
        let (q, r, o) = search.with_untracked(|s| (s.query.clone(), s.replace.clone(), s.opts));
        let before = h.sel_full();
        h.core.update_value(|c| {
            let snap = CodeEditorHandle::snapshot(c, before);
            CodeEditorHandle::push_undo(c, snap);
            c.last_kind = None;
            for &(a, b) in ms.iter().rev() {
                let rep = replacement_for(&c.doc.text[a..b], &q, &r, o);
                c.doc.apply(a, b, &rep);
            }
        });
        let len = h.core.with_value(|c| c.doc.text.len());
        let x = before.0.min(len);
        h.sync(Some((x, x)));
        h.emit_change();
        search.update(|s| s.current = None);
    };

    // ---- Eingabe ----
    let on_beforeinput = move |e: web_sys::InputEvent| {
        let kind = e.input_type();
        if kind == "historyUndo" || kind == "historyRedo" {
            e.prevent_default();
            h.undo_redo(kind == "historyRedo");
            return;
        }
        let sel = h.sel16();
        h.core.update_value(|c| c.sel_before = sel);
    };
    let on_input = move |e: web_sys::Event| {
        let Some(ta) = textarea.get_untracked() else { return };
        let (kind, data) = e
            .dyn_ref::<web_sys::InputEvent>()
            .map(|ie| (ie.input_type(), ie.data().unwrap_or_default()))
            .unwrap_or_default();
        let new_view = ta.value();
        let now = js_sys::Date::now();
        let res = h.core.try_update_value(|c| {
            let old_view = std::mem::take(&mut c.view);
            let sel_b = c.sel_before.take();
            let before_full = sel_b
                .map(|(s, e)| {
                    let (vs, ve) = (utf16_to_byte(&old_view, s), utf16_to_byte(&old_view, e));
                    (c.doc.to_full(vs, false), c.doc.to_full(ve, false))
                })
                .unwrap_or((0, 0));
            let mergeable = matches!(kind.as_str(), "insertText" | "deleteContentBackward" | "deleteContentForward")
                && !c.accepting;
            let merge = mergeable
                && c.last_kind.as_ref().is_some_and(|(k, t)| *k == kind && now - t < UNDO_MERGE_MS)
                && !c.undo.is_empty();
            if merge {
                c.redo.clear();
            } else {
                let snap = CodeEditorHandle::snapshot(c, before_full);
                CodeEditorHandle::push_undo(c, snap);
            }
            c.last_kind = mergeable.then(|| (kind.clone(), now));
            let caret_before = sel_b.map(|(s, _)| utf16_to_byte(&old_view, s));
            let end_full = c.doc.apply_view_edit(&old_view, &new_view, caret_before);
            c.view = c.doc.view();
            let caret16 = byte_to_utf16(&c.view, c.doc.to_view(end_full));
            (c.view != new_view).then(|| (c.view.clone(), caret16))
        });
        // Eine Faltung wurde aufgelöst -> Textarea angleichen.
        if let Some(Some((view, caret))) = res {
            ta.set_value(&view);
            let _ = ta.set_selection_range(caret, caret);
        }
        rev.update(|r| *r += 1);
        h.emit_change();
        update_active();
        let accepting = h.core.with_value(|c| c.accepting);
        if accepting {
            return;
        }
        let typed_ident = kind == "insertText" && data.chars().all(|ch| ch.is_alphanumeric() || ch == '_' || ch == '.');
        if (typed_ident && !data.is_empty()) || (kind.starts_with("delete") && comp.with_untracked(Option::is_some)) {
            open_completion(false);
        } else {
            comp.set(None);
        }
    };

    let on_keydown = move |e: web_sys::KeyboardEvent| {
        let key = e.key();
        let code = e.code();
        let ctrl = e.ctrl_key() || e.meta_key();
        let (shift, alt) = (e.shift_key(), e.alt_key());
        if comp.with_untracked(Option::is_some) {
            match key.as_str() {
                "ArrowDown" => {
                    e.prevent_default();
                    move_completion(1);
                    return;
                }
                "ArrowUp" => {
                    e.prevent_default();
                    move_completion(-1);
                    return;
                }
                "Enter" | "Tab" if !ctrl && !shift && !alt => {
                    e.prevent_default();
                    let i = comp.with_untracked(|c| c.as_ref().map(|c| c.sel).unwrap_or(0));
                    accept_completion(i);
                    return;
                }
                "Escape" => {
                    e.prevent_default();
                    comp.set(None);
                    return;
                }
                "ArrowLeft" | "ArrowRight" | "Home" | "End" | "PageUp" | "PageDown" => comp.set(None),
                _ => {}
            }
        }
        let lower = key.to_lowercase();
        if ctrl && !alt && lower == "f" {
            e.prevent_default();
            open_search();
        } else if key == "F3" {
            e.prevent_default();
            if search.with_untracked(|s| s.open) {
                find_next(shift);
            } else {
                open_search();
            }
        } else if ctrl && !alt && lower == "z" {
            e.prevent_default();
            h.undo_redo(shift);
        } else if ctrl && !alt && lower == "y" {
            e.prevent_default();
            h.undo_redo(true);
        } else if ctrl && (key == " " || code == "Space") {
            e.prevent_default();
            open_completion(true);
        } else if ctrl && shift && !alt && code == "BracketLeft" {
            e.prevent_default();
            h.fold_at_cursor(true);
        } else if ctrl && shift && !alt && code == "BracketRight" {
            e.prevent_default();
            h.fold_at_cursor(false);
        } else if ctrl && alt && code == "BracketLeft" {
            e.prevent_default();
            h.fold_all(true);
        } else if ctrl && alt && code == "BracketRight" {
            e.prevent_default();
            h.fold_all(false);
        } else if key == "Escape" && search.with_untracked(|s| s.open) {
            e.prevent_default();
            close_search();
        } else if key == "Tab" && !shift && !ctrl && !alt {
            e.prevent_default();
            h.insert_at_cursor("\t");
        } else {
            let sel = h.sel16();
            h.core.update_value(|c| c.sel_before = sel);
        }
    };

    // ---- Darstellung ----
    let gutter_html = Memo::new(move |_| {
        let (vis, folded) = layout.get();
        let total = text_m.with(|t| t.matches('\n').count() + 1);
        let digits = total.to_string().len();
        ranges.with(|r| {
            let mut out = String::with_capacity(vis.len() * 120);
            for l in vis {
                let marker = if folded.contains(&l) {
                    "<span class=\"cursor-pointer text-foreground\" data-fold=\"L\" title=\"Aufklappen\">▸</span>"
                } else if r.binary_search_by_key(&l, |x| x.0).is_ok() {
                    "<span class=\"cursor-pointer opacity-60 hover:opacity-100\" data-fold=\"L\" title=\"Zuklappen\">▾</span>"
                } else {
                    ""
                };
                out.push_str(&format!(
                    "<div class=\"flex\" style=\"height:{LINE_HEIGHT}px\"><span class=\"pl-2 text-right\" style=\"width:{}ch\">{}</span><span class=\"w-4 text-center\">{}</span></div>",
                    digits + 1,
                    l + 1,
                    marker.replace('L', &l.to_string())
                ));
            }
            out
        })
    });
    // Zeilenumbruch am Ende würde vom <pre> verschluckt - eine Leerzeile
    // anhängen, damit Overlay und Textarea gleich hoch sind.
    let highlighted = Memo::new(move |_| {
        let (vis, folded) = layout.get();
        lines_html.with(|lh| {
            let mut out = String::new();
            for (i, l) in vis.iter().enumerate() {
                if i > 0 {
                    out.push('\n');
                }
                out.push_str(lh.get(*l).map(String::as_str).unwrap_or(""));
                if folded.contains(l) {
                    out.push_str(FOLD_PLACEHOLDER);
                }
            }
            out.push_str("\n ");
            out
        })
    });
    // Suchtreffer als <mark> unter dem Text.
    let marks_html = Memo::new(move |_| {
        let current = search.with(|s| s.current);
        let ms = match matches.get() {
            Ok(m) if !m.is_empty() => m,
            _ => return String::new(),
        };
        rev.track();
        h.core.with_value(|c| {
            let view = &c.view;
            let mut out = String::with_capacity(view.len() + ms.len() * 60);
            let mut pos = 0;
            for (i, &(a, b)) in ms.iter().enumerate() {
                let (va, vb) = (c.doc.to_view(a), c.doc.to_view(b));
                if vb - va != b - a || va < pos || vb > view.len() {
                    continue; // (teilweise) eingeklappt
                }
                out.push_str(&escape_html(&view[pos..va]));
                let cls = if current == Some(i) { "bg-orange-400/60" } else { "bg-yellow-300/50 dark:bg-yellow-500/30" };
                out.push_str(&format!("<mark class=\"rounded-sm text-transparent {cls}\">"));
                out.push_str(&escape_html(&view[va..vb]));
                out.push_str("</mark>");
                pos = vb;
            }
            out.push_str(&escape_html(&view[pos..]));
            out.push_str("\n ");
            out
        })
    });
    let match_info = move || match matches.get() {
        Err(e) => e,
        Ok(m) if search.with(|s| s.query.is_empty()) => {
            let _ = m;
            String::new()
        }
        Ok(m) if m.is_empty() => "Keine Treffer".into(),
        Ok(m) => {
            let more = if m.len() >= MAX_MATCHES { "+" } else { "" };
            match search.with(|s| s.current) {
                Some(i) => format!("{} von {}{more}", i + 1, m.len()),
                None => format!("{}{more} Treffer", m.len()),
            }
        }
    };

    let on_gutter_click = move |e: web_sys::MouseEvent| {
        let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
        let Some(line) = el.closest("[data-fold]").ok().flatten().and_then(|f| f.get_attribute("data-fold")) else {
            return;
        };
        if let Ok(l) = line.parse::<usize>() {
            e.prevent_default();
            comp.set(None);
            h.toggle_fold(l);
        }
    };

    let toggle_btn = move |label: &'static str, title: &'static str, get: fn(&SearchOpts) -> bool, set: fn(&mut SearchOpts)| {
        view! {
            <button
                type="button"
                title=title
                class=move || {
                    if search.with(|s| get(&s.opts)) {
                        "h-7 rounded border border-primary bg-primary/15 px-1.5 font-mono"
                    } else {
                        "h-7 rounded border border-transparent px-1.5 font-mono text-muted-foreground hover:bg-muted"
                    }
                }
                on:mousedown=|e| e.prevent_default()
                on:click=move |_| search.update(|s| {
                    set(&mut s.opts);
                    s.current = None;
                })
            >
                {label}
            </button>
        }
    };
    let find_keydown = move |e: web_sys::KeyboardEvent| match e.key().as_str() {
        "Enter" | "F3" => {
            e.prevent_default();
            find_next(e.shift_key());
        }
        "Escape" => {
            e.prevent_default();
            close_search();
        }
        k if (e.ctrl_key() || e.meta_key()) && k.eq_ignore_ascii_case("f") => {
            e.prevent_default();
            if let Some(inp) = find_input.get_untracked() {
                inp.select();
            }
        }
        _ => {}
    };
    let replace_keydown = move |e: web_sys::KeyboardEvent| match e.key().as_str() {
        "Enter" => {
            e.prevent_default();
            replace_one();
        }
        "Escape" => {
            e.prevent_default();
            close_search();
        }
        _ => {}
    };

    let search_panel = move || {
        view! {
            <div class="flex shrink-0 flex-wrap items-center gap-1.5 border-t border-border bg-muted/30 px-2 py-1.5 font-sans text-xs">
                <input
                    node_ref=find_input
                    type="text"
                    placeholder="Suchen"
                    spellcheck="false"
                    class="h-7 w-40 min-w-24 shrink rounded border border-input bg-background px-2 outline-none focus:border-primary"
                    prop:value=move || search.with(|s| s.query.clone())
                    on:input=move |e| {
                        let v = event_target_value(&e);
                        search.update(|s| {
                            s.query = v;
                            s.current = None;
                        });
                    }
                    on:keydown=find_keydown
                />
                <button type="button" title="Nächster Treffer (Enter, F3)" class="h-7 rounded px-1 hover:bg-muted" on:click=move |_| find_next(false)>
                    <Icon name="ArrowDown" class="size-3.5"/>
                </button>
                <button type="button" title="Vorheriger Treffer (Umschalt+Enter)" class="h-7 rounded px-1 hover:bg-muted" on:click=move |_| find_next(true)>
                    <Icon name="ArrowUp" class="size-3.5"/>
                </button>
                {toggle_btn("Aa", "Groß-/Kleinschreibung beachten", |o| o.case, |o| o.case = !o.case)}
                {toggle_btn("ab", "Nur ganze Wörter", |o| o.word, |o| o.word = !o.word)}
                {toggle_btn(".*", "Regulärer Ausdruck", |o| o.regex, |o| o.regex = !o.regex)}
                <span class=move || if matches.with(|m| m.is_err()) { "min-w-16 whitespace-nowrap text-destructive" } else { "min-w-16 whitespace-nowrap text-muted-foreground" }>
                    {match_info}
                </span>
                <input
                    type="text"
                    placeholder="Ersetzen"
                    spellcheck="false"
                    class="h-7 w-32 min-w-20 shrink rounded border border-input bg-background px-2 outline-none focus:border-primary"
                    prop:value=move || search.with(|s| s.replace.clone())
                    on:input=move |e| {
                        let v = event_target_value(&e);
                        search.update(|s| s.replace = v);
                    }
                    on:keydown=replace_keydown
                />
                <button type="button" class="h-7 whitespace-nowrap rounded border border-border px-2 hover:bg-muted" on:click=move |_| replace_one()>
                    "Ersetzen"
                </button>
                <button type="button" class="h-7 whitespace-nowrap rounded border border-border px-2 hover:bg-muted" on:click=move |_| replace_all()>
                    "Alle ersetzen"
                </button>
                <button type="button" title="Schließen (Esc)" class="ml-auto h-7 rounded px-1 hover:bg-muted" on:click=move |_| close_search()>
                    <Icon name="X" class="size-3.5"/>
                </button>
            </div>
        }
    };

    let completion_popup = move || {
        comp.get().map(|ui| {
            let (t, l) = scroll.get();
            let items = ui
                .items
                .iter()
                .enumerate()
                .map(|(i, it)| {
                    let cls = if i == ui.sel {
                        "flex cursor-pointer items-center gap-3 bg-primary/15 px-2 py-0.5"
                    } else {
                        "flex cursor-pointer items-center gap-3 px-2 py-0.5 hover:bg-muted"
                    };
                    view! {
                        <li
                            class=cls
                            on:mousedown=move |e: web_sys::MouseEvent| {
                                e.prevent_default();
                                accept_completion(i);
                            }
                        >
                            <span class="flex-1 truncate">{it.label.clone()}</span>
                            <span class="shrink-0 font-sans text-[10px] text-muted-foreground">{it.kind.label()}</span>
                        </li>
                    }
                })
                .collect_view();
            view! {
                <ul
                    node_ref=comp_list
                    class="absolute z-20 max-h-56 w-80 overflow-auto rounded-md border border-border bg-popover py-1 text-popover-foreground shadow-md"
                    style=format!("top:{}px;left:{}px;", ui.top - t, (ui.left - l).max(0.0))
                >
                    {items}
                </ul>
            }
        })
    };

    view! {
        <div class=move || format!("flex h-full flex-col overflow-hidden bg-background font-mono text-xs {}", class.get().unwrap_or_default())>
            <div class="relative flex min-h-0 flex-1 overflow-hidden">
                // Zeilennummern + Faltpfeile
                <div
                    class="relative shrink-0 overflow-hidden border-r border-border bg-muted/30 text-muted-foreground select-none"
                    on:mousedown=on_gutter_click
                >
                    <div
                        style=move || format!("padding:{PAD_Y}px 0;transform:translateY({}px);", -scroll.get().0)
                        inner_html=move || gutter_html.get()
                    ></div>
                </div>
                <div class="relative min-w-0 flex-1 overflow-hidden">
                    // aktive Zeile
                    <div
                        aria-hidden="true"
                        class=move || if focused.get() { "pointer-events-none absolute inset-x-0 bg-muted/60" } else { "pointer-events-none absolute inset-x-0 bg-muted/30" }
                        style=move || format!("height:{LINE_HEIGHT}px;top:{}px;", PAD_Y + active_line.get() as f64 * LINE_HEIGHT - scroll.get().0)
                    ></div>
                    // Suchtreffer
                    <pre
                        aria-hidden="true"
                        class="pointer-events-none absolute left-0 top-0 min-w-full whitespace-pre font-mono text-transparent"
                        style=move || {
                            let (t, l) = scroll.get();
                            format!("{TEXT_STYLE}transform:translate({}px,{}px);", -l, -t)
                        }
                        inner_html=move || marks_html.get()
                    ></pre>
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
                    // Zeichenbreite messen (für die Position der Vorschlagsliste)
                    <span node_ref=measure aria-hidden="true" class="invisible absolute whitespace-pre font-mono">"0000000000"</span>
                    // Eingabe (Text unsichtbar, nur Cursor und Auswahl sichtbar)
                    <textarea
                        node_ref=textarea
                        wrap="off"
                        spellcheck="false"
                        autocomplete="off"
                        autocapitalize="off"
                        class="absolute inset-0 h-full w-full resize-none overflow-auto whitespace-pre border-0 bg-transparent font-mono text-transparent caret-foreground outline-none selection:bg-primary/25"
                        style=TEXT_STYLE
                        on:beforeinput=on_beforeinput
                        on:input=on_input
                        on:scroll=move |_| sync_scroll()
                        on:keydown=on_keydown
                        on:keyup=move |_| update_active()
                        on:click=move |_| {
                            comp.set(None);
                            update_active();
                        }
                        on:select=move |_| update_active()
                        on:focus=move |_| {
                            focused.set(true);
                            update_active();
                        }
                        on:blur=move |_| {
                            focused.set(false);
                            comp.set(None);
                        }
                    ></textarea>
                    {completion_popup}
                </div>
            </div>
            <Show when=move || search.with(|s| s.open)>{search_panel}</Show>
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

    #[test]
    fn columns_expand_tabs() {
        assert_eq!(display_cols("ab"), 2);
        assert_eq!(display_cols("\tx"), 5);
        assert_eq!(display_cols("ab\tx"), 5);
    }
}
