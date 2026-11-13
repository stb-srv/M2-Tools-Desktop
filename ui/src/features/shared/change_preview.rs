//! Änderungsvorschau vor dem Hochladen einer Drop-Datei: was steht jetzt
//! auf dem Server, was würde gespeichert? Holt beides über
//! `preview_drop_file` (schreibt nichts) und zeigt nur die geänderten
//! Stellen mit etwas Kontext.

use crate::components::Icon;
use crate::features::backup_browser::diff::{diff_lines, PartKind};
use crate::tauri::invoke;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use serde_json::json;

#[derive(Clone, Debug, Deserialize)]
struct FilePreview {
    #[allow(dead_code)]
    file: String,
    path: String,
    old: Option<String>,
    new: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Row {
    Line { kind: PartKind, text: String },
    /// So viele unveränderte Zeilen ausgelassen.
    Skipped(usize),
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Summary {
    pub added: usize,
    pub removed: usize,
    pub rows: Vec<Row>,
}

/// Geänderte Zeilen mit `context` unveränderten Zeilen davor und danach.
pub fn summarize(old: &str, new: &str, context: usize) -> Summary {
    let parts = diff_lines(old, new);
    let mut out = Summary::default();
    let n = parts.len();
    for (i, p) in parts.into_iter().enumerate() {
        match p.kind {
            PartKind::Added => out.added += p.lines.len(),
            PartKind::Removed => out.removed += p.lines.len(),
            PartKind::Unchanged => {}
        }
        if p.kind != PartKind::Unchanged {
            out.rows.extend(p.lines.into_iter().map(|text| Row::Line { kind: p.kind, text }));
            continue;
        }
        let len = p.lines.len();
        let keep_head = if i == 0 { 0 } else { context.min(len) };
        let keep_tail = if i + 1 == n { 0 } else { context.min(len) };
        if keep_head + keep_tail >= len {
            out.rows.extend(p.lines.into_iter().map(|text| Row::Line { kind: PartKind::Unchanged, text }));
        } else {
            let skipped = len - keep_head - keep_tail;
            let mut lines = p.lines.into_iter();
            out.rows.extend(lines.by_ref().take(keep_head).map(|text| Row::Line { kind: PartKind::Unchanged, text }));
            out.rows.push(Row::Skipped(skipped));
            out.rows.extend(lines.skip(skipped).map(|text| Row::Line { kind: PartKind::Unchanged, text }));
        }
    }
    out
}

fn row_class(kind: PartKind) -> &'static str {
    match kind {
        PartKind::Added => "bg-green-500/15 text-green-700 dark:text-green-400",
        PartKind::Removed => "bg-red-500/15 text-red-700 dark:text-red-400",
        PartKind::Unchanged => "text-muted-foreground",
    }
}

/// `kind`: "mob_drop" | "common_drop" | "etc_drop" | "special_item_group" |
/// "drop_item_group"; `data`: genau das, was das Speichern bekommt.
#[component]
pub fn ChangePreview(kind: &'static str, data: serde_json::Value) -> impl IntoView {
    let result = RwSignal::new(None::<Result<(String, bool, Summary), String>>);
    spawn_local(async move {
        let res = invoke::<FilePreview>("preview_drop_file", &json!({ "kind": kind, "data": data })).await.map(|p| {
            let is_new = p.old.is_none();
            (p.path, is_new, summarize(p.old.as_deref().unwrap_or(""), &p.new, 2))
        });
        result.set(Some(res));
    });
    view! {
        <div class="space-y-1">
            {move || match result.get() {
                None => view! {
                    <p class="flex items-center gap-1 text-xs text-muted-foreground">
                        <Icon name="Loader2" class="size-3.5 animate-spin"/>
                        "Vergleiche mit der Datei auf dem Server…"
                    </p>
                }.into_any(),
                Some(Err(e)) => view! {
                    <p class="text-xs text-amber-600 dark:text-amber-400">"Keine Vorschau möglich: " {e}</p>
                }.into_any(),
                Some(Ok((path, is_new, s))) => {
                    let unchanged = s.added == 0 && s.removed == 0;
                    view! {
                        <p class="text-xs text-muted-foreground">
                            {if is_new { format!("{path} wird neu angelegt.") } else { format!("Änderungen an {path}:") }}
                            " "
                            <span class="text-green-600">{format!("+{}", s.added)}</span>
                            " / "
                            <span class="text-destructive">{format!("−{}", s.removed)}</span>
                            " Zeilen"
                        </p>
                        {if unchanged {
                            view! { <p class="text-xs text-muted-foreground">"Keine Änderung gegenüber dem Server."</p> }.into_any()
                        } else {
                            view! {
                                <pre class="max-h-64 overflow-auto rounded-md border border-border bg-muted/30 p-2 font-mono text-[11px] leading-snug">
                                    {s.rows.into_iter().map(|r| match r {
                                        Row::Line { kind, text } => {
                                            let prefix = match kind { PartKind::Added => "+ ", PartKind::Removed => "- ", PartKind::Unchanged => "  " };
                                            view! { <div class=row_class(kind)>{prefix}{text.replace('\t', "→")}</div> }.into_any()
                                        }
                                        Row::Skipped(n) => view! {
                                            <div class="py-0.5 text-muted-foreground/70">{format!("… {n} unveränderte Zeile(n) …")}</div>
                                        }.into_any(),
                                    }).collect_view()}
                                </pre>
                            }.into_any()
                        }}
                    }.into_any()
                }
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_and_collapses_context() {
        let old: String = (1..=20).map(|i| format!("l{i}\n")).collect();
        let new = old.replace("l10\n", "l10x\n");
        let s = summarize(&old, &new, 2);
        assert_eq!((s.added, s.removed), (1, 1));
        assert_eq!(s.rows.first(), Some(&Row::Skipped(7)));
        assert_eq!(s.rows.last(), Some(&Row::Skipped(8)));
        assert_eq!(s.rows.len(), 2 + 2 + 2 + 2);
    }

    #[test]
    fn new_file_is_all_added() {
        let s = summarize("", "a\nb\n", 2);
        assert_eq!((s.added, s.removed), (2, 0));
        assert!(s.rows.iter().all(|r| matches!(r, Row::Line { kind: PartKind::Added, .. })));
    }

    #[test]
    fn unchanged() {
        let s = summarize("a\n", "a\n", 2);
        assert_eq!((s.added, s.removed), (0, 0));
    }
}
