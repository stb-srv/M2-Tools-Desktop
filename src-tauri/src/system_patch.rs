// Parser für die "System"-Paket-Konvention, wie sie in echten Community-
// Metin2-"Systemen" (z.B. Admin-Panel-Module, ResizeWindow) verwendet wird -
// gegen zwei reale Beispielsysteme verifiziert (nicht angenommen), zwei
// unabhängige Autoren, erstaunlich einheitliche Konvention:
//
//   // search
//   <vorhandener Code als Suchtext>
//   // add above|below|inside|at the end
//   <neuer Code>
//
// Varianten real gefunden: `// search`, `//search:`, `## search:`/`# search`
// (Python), `// add above`, `// add below`, `// add ABOVE, not below:`,
// `##inside:` (setzt einen Scope-Anker für die folgenden Blöcke, bis ein
// neuer `inside`-Marker kommt), `##add at the end:` (Ende des aktuellen
// Scopes, kein eigener Suchtext nötig), bloßes `// add` (Standard: unten).
// Daneben: Freitext-Anweisungen ohne festen Suchtext ("search and edit like
// this", "add in MODULES_DICT and set an ID") - dafür gibt es keinen
// zuverlässigen Automatismus, die werden als `FreeformInstruction` markiert
// statt geraten. Eine Datei ganz ohne jede Markierung liefert eine leere
// `ops`-Liste - der Aufrufer behandelt das als Ganzdatei-Kandidat (neue
// Datei).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Placement {
    Above,
    Below,
    Inside,
    AtEnd,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum PatchOp {
    SearchInsert {
        /// Optionaler äußerer Anker (aus einem vorangehenden `inside`-
        /// Marker) - grenzt die Suche auf einen Bereich nach dieser Stelle
        /// ein, damit z.B. `self.SetSize(...)` nicht in der falschen
        /// Funktion gefunden wird.
        scope: Option<String>,
        /// Leer, wenn `placement == AtEnd` ohne eigenen Suchtext (z.B.
        /// "add at the end" direkt nach einem `inside`-Marker).
        anchor: String,
        placement: Placement,
        code: String,
    },
    FreeformInstruction {
        instruction: String,
        code: String,
    },
    AppendToEnd {
        code: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedSystemFile {
    pub ops: Vec<PatchOp>,
}

// ---- Anker-Suche ----
//
// Löst das vom Nutzer genannte Problem: manche Suchtexte matchen nicht
// 1:1, weil ein Kommentar mittendrin in der echten Zieldatei anders
// aussieht (z.B. eine andere Formulierung/Version). Statt einmalig exakt
// zu vergleichen und bei Misserfolg aufzugeben, wird stufenweise
// toleranter gesucht - jede Stufe wird in der UI sichtbar gemacht, damit
// klar ist, warum etwas automatisch übernommen wurde oder nicht.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnchorConfidence {
    /// Zeilen exakt gleich (nur Zeilenende-Whitespace ignoriert).
    Exact,
    /// Einrückung (Tabs/Spaces) ignoriert.
    WhitespaceTolerant,
    /// Reine Kommentarzeilen an beiden Seiten ignoriert (Inhalt kann
    /// abweichen, nur dass an dieser Stelle überhaupt ein Kommentar steht
    /// zählt).
    CommentStripped,
    /// Nur die letzte, nicht-Kommentar-Zeile des Ankers eindeutig
    /// gefunden - deutlich unsicherer, wird nie wie ein exakter Treffer
    /// automatisch übernommen.
    Partial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineRange {
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum AnchorMatch {
    Found { range: LineRange, confidence: AnchorConfidence },
    Ambiguous { candidates: Vec<LineRange> },
    NotFound,
}

fn is_comment_only(line: &str) -> bool {
    let t = line.trim();
    t.is_empty() || t.starts_with("//") || t.starts_with('#')
}

fn find_contiguous(
    haystack: &[&str],
    anchor: &[&str],
    eq: impl Fn(&str, &str) -> bool,
) -> Vec<LineRange> {
    if anchor.is_empty() || haystack.len() < anchor.len() {
        return vec![];
    }
    (0..=(haystack.len() - anchor.len()))
        .filter(|&start| (0..anchor.len()).all(|k| eq(haystack[start + k], anchor[k])))
        .map(|start| LineRange { start_line: start, end_line: start + anchor.len() - 1 })
        .collect()
}

fn matches_at_confidence(haystack: &[&str], anchor: &[&str], confidence: AnchorConfidence) -> Vec<LineRange> {
    match confidence {
        AnchorConfidence::Exact => find_contiguous(haystack, anchor, |h, a| h.trim_end() == a.trim_end()),
        AnchorConfidence::WhitespaceTolerant => find_contiguous(haystack, anchor, |h, a| h.trim() == a.trim()),
        AnchorConfidence::CommentStripped => find_contiguous(haystack, anchor, |h, a| {
            (is_comment_only(h) && is_comment_only(a)) || h.trim() == a.trim()
        }),
        AnchorConfidence::Partial => {
            let Some(last_code_line) = anchor.iter().rev().find(|l| !is_comment_only(l)) else {
                return vec![];
            };
            find_contiguous(haystack, std::slice::from_ref(last_code_line), |h, a| h.trim() == a.trim())
        }
    }
}

/// Sucht `anchor` (mehrzeiliger Text) in `haystack` (kompletter Inhalt der
/// echten Zieldatei), stufenweise toleranter. Ein leerer Anker (z.B.
/// `Placement::AtEnd` ohne eigenen Suchtext) hat hier nichts zu suchen -
/// das behandelt der Aufrufer separat.
pub fn find_anchor(haystack: &str, anchor: &str) -> AnchorMatch {
    let haystack_lines: Vec<&str> = haystack.lines().collect();
    let anchor_lines: Vec<&str> = anchor.lines().collect();
    if anchor_lines.is_empty() {
        return AnchorMatch::NotFound;
    }

    for confidence in [
        AnchorConfidence::Exact,
        AnchorConfidence::WhitespaceTolerant,
        AnchorConfidence::CommentStripped,
        AnchorConfidence::Partial,
    ] {
        let candidates = matches_at_confidence(&haystack_lines, &anchor_lines, confidence);
        match candidates.len() {
            0 => continue,
            1 => return AnchorMatch::Found { range: candidates[0], confidence },
            _ => return AnchorMatch::Ambiguous { candidates },
        }
    }
    AnchorMatch::NotFound
}

fn leading_whitespace(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Grenzt die Suche auf den Bereich nach dem Scope-Anker ein - bis zur
/// nächsten Zeile auf gleicher oder geringerer Einrückung (funktioniert gut
/// für einrückungsbasierte Sprachen wie Python) oder spätestens nach einer
/// großzügigen Zeilenobergrenze (Sicherheitsnetz für Sprachen wie C++, wo
/// diese Heuristik nicht greift). Gibt Start-/Ende-Zeilenindex relativ zu
/// `haystack_lines` zurück.
fn scope_window(haystack_lines: &[&str], scope_end: usize, scope_indent: usize) -> (usize, usize) {
    const MAX_WINDOW: usize = 300;
    let window_start = scope_end + 1;
    let cap = haystack_lines.len().min(window_start + MAX_WINDOW);
    let mut window_end = cap;
    for (offset, line) in haystack_lines[window_start..cap].iter().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        if leading_whitespace(line) <= scope_indent {
            window_end = window_start + offset;
            break;
        }
    }
    (window_start, window_end)
}

/// Wie `find_anchor`, aber wenn `scope` gesetzt ist (aus einem
/// vorangehenden `inside`-Marker), wird zuerst der Scope-Anker gesucht und
/// die eigentliche Anker-Suche auf den Bereich danach eingegrenzt - löst
/// z.B. "self.SetSize(...) kommt in mehreren Funktionen vor, aber ich
/// meinte die in Open(self)".
pub fn find_anchor_with_scope(haystack: &str, scope: Option<&str>, anchor: &str) -> AnchorMatch {
    let Some(scope_text) = scope else {
        return find_anchor(haystack, anchor);
    };
    let haystack_lines: Vec<&str> = haystack.lines().collect();
    let AnchorMatch::Found { range: scope_range, .. } = find_anchor(haystack, scope_text) else {
        // Scope selbst nicht eindeutig auffindbar - ohne verlässlichen
        // Ausgangspunkt keine sinnvolle Eingrenzung möglich, das braucht
        // manuelle Prüfung statt eines geratenen Fensters.
        return AnchorMatch::NotFound;
    };
    let scope_indent = leading_whitespace(haystack_lines[scope_range.start_line]);
    let (window_start, window_end) = scope_window(&haystack_lines, scope_range.end_line, scope_indent);
    let window_text = haystack_lines[window_start..window_end].join("\n");

    match find_anchor(&window_text, anchor) {
        AnchorMatch::Found { range, confidence } => AnchorMatch::Found {
            range: LineRange {
                start_line: range.start_line + window_start,
                end_line: range.end_line + window_start,
            },
            confidence,
        },
        AnchorMatch::Ambiguous { candidates } => AnchorMatch::Ambiguous {
            candidates: candidates
                .into_iter()
                .map(|c| LineRange { start_line: c.start_line + window_start, end_line: c.end_line + window_start })
                .collect(),
        },
        AnchorMatch::NotFound => AnchorMatch::NotFound,
    }
}

/// Ergebnis von `resolve_insertion` - entweder ein konkreter, sicher genug
/// bestimmter Einfüge-Zeilenindex, oder ein Grund, warum das nicht
/// automatisch geht (wird in der UI als "Prüfung nötig" angezeigt statt
/// blind übernommen zu werden).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum InsertionResolution {
    Ready { line: usize, confidence: AnchorConfidence },
    NeedsReview { reason: String },
}

/// Bestimmt, an welcher Zeile (0-basiert) `code` in `haystack` eingefügt
/// werden müsste. Ein leerer `anchor` bedeutet "kein eigener Suchtext nötig"
/// (z.B. "add at the end" direkt nach einem `inside`-Marker) - der
/// Einfügepunkt kommt dann direkt aus dem Scope-Fenster (oder Dateiende,
/// wenn kein Scope gesetzt ist) statt aus einer Anker-Suche.
pub fn resolve_insertion(
    haystack: &str,
    scope: Option<&str>,
    anchor: &str,
    placement: Placement,
) -> InsertionResolution {
    if anchor.is_empty() {
        let haystack_lines: Vec<&str> = haystack.lines().collect();
        let line = match scope {
            Some(scope_text) => match find_anchor(haystack, scope_text) {
                AnchorMatch::Found { range, .. } => {
                    let indent = leading_whitespace(haystack_lines[range.start_line]);
                    let (_, end) = scope_window(&haystack_lines, range.end_line, indent);
                    end
                }
                _ => {
                    return InsertionResolution::NeedsReview {
                        reason: "Scope-Anker nicht eindeutig gefunden".to_string(),
                    };
                }
            },
            None => haystack_lines.len(),
        };
        return InsertionResolution::Ready { line, confidence: AnchorConfidence::Exact };
    }

    match find_anchor_with_scope(haystack, scope, anchor) {
        AnchorMatch::Found { range, confidence } => {
            let line = match placement {
                Placement::Above => range.start_line,
                Placement::Below | Placement::Inside | Placement::AtEnd => range.end_line + 1,
            };
            InsertionResolution::Ready { line, confidence }
        }
        AnchorMatch::Ambiguous { .. } => {
            InsertionResolution::NeedsReview { reason: "Suchtext mehrdeutig - mehrere Stellen gefunden".to_string() }
        }
        AnchorMatch::NotFound => {
            InsertionResolution::NeedsReview { reason: "Suchtext nicht gefunden".to_string() }
        }
    }
}

/// Fügt `code` (kann mehrzeilig sein) vor Zeile `line` (0-basiert) in
/// `content` ein. `line == content.lines().count()` fügt am Ende an.
pub fn splice_lines(content: &str, line: usize, code: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let insert_lines: Vec<&str> = code.lines().collect();
    let at = line.min(lines.len());

    let mut result: Vec<&str> = Vec::with_capacity(lines.len() + insert_lines.len());
    result.extend_from_slice(&lines[..at]);
    result.extend_from_slice(&insert_lines);
    result.extend_from_slice(&lines[at..]);

    let mut joined = result.join("\n");
    if content.is_empty() || content.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

/// Einfacher Struktur-Check als Sicherheitsnetz nach dem automatischen
/// Einbau (bewusst kein vollständiger Syntax-Checker - siehe Modul-Planung:
/// das ist ein eigenständiges, größeres Folge-Projekt). Prüft nur, ob die
/// **fertige** Datei in sich balanciert ist - Klammern zählen sich
/// unabhängig vom eigentlichen Einbau-Vorgang, ein bereits vorher
/// unbalanciertes Original würde also auch schon vorher auffallen.
/// Gibt `None` zurück, wenn alles ausgeglichen aussieht (oder die
/// Dateiendung nicht geprüft wird).
pub fn check_structural_balance(filename: &str, content: &str) -> Option<String> {
    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    if !["cpp", "c", "h", "hpp"].contains(&ext.as_str()) {
        return None;
    }
    let opens = content.matches('{').count();
    let closes = content.matches('}').count();
    if opens != closes {
        return Some(format!("Geschweifte Klammern nicht ausgeglichen ({{: {opens}, }}: {closes})"));
    }
    let popens = content.matches('(').count();
    let pcloses = content.matches(')').count();
    if popens != pcloses {
        return Some(format!("Runde Klammern nicht ausgeglichen ((: {popens}, ): {pcloses})"));
    }
    let if_count = content.lines().filter(|l| l.trim_start().starts_with("#if")).count();
    let endif_count = content.lines().filter(|l| l.trim_start().starts_with("#endif")).count();
    if if_count != endif_count {
        return Some(format!("#if/#endif nicht ausgeglichen (#if*: {if_count}, #endif: {endif_count})"));
    }
    None
}

#[derive(Debug, Clone, PartialEq)]
enum Marker {
    Search,
    Add(Placement),
    /// Eine Kopfzeile, die zwar wie eine Anweisung aussieht ("search and
    /// edit like this", "add in MODULES_DICT and set an ID for you"), aber
    /// keinen festen Suchtext/keine feste Richtung hat - kein
    /// zuverlässiger Automatismus möglich, wird als Ganzes zur Prüfung
    /// vorgelegt.
    Freeform(String),
    Inside,
}

/// Erkennt eine Markierungszeile - toleriert `//`, `##`, `#` als
/// Kommentarpräfix und einen optionalen Doppelpunkt. Gibt `None` zurück für
/// jede normale Code-/Textzeile.
fn classify_marker_line(line: &str) -> Option<Marker> {
    let trimmed = line.trim();
    let stripped = trimmed
        .strip_prefix("##")
        .or_else(|| trimmed.strip_prefix("//"))
        .or_else(|| trimmed.strip_prefix('#'))?;
    let body = stripped.trim().trim_end_matches(':').trim();
    let lower = body.to_lowercase();

    if lower == "search" {
        return Some(Marker::Search);
    }
    if lower.starts_with("search") {
        // z.B. "search and edit like this" - kein fester Suchtext.
        return Some(Marker::Freeform(trimmed.to_string()));
    }
    if lower == "inside" {
        return Some(Marker::Inside);
    }
    if lower.starts_with("add") {
        let rest = lower.strip_prefix("add").unwrap_or("").trim();
        // Reihenfolge wichtig: "add ABOVE, not below" muss als Above
        // erkannt werden, nicht als Below, weil "below" später im Text
        // auch vorkommt - erstes gefundenes Richtungswort gewinnt.
        if rest.is_empty() {
            return Some(Marker::Add(Placement::Below));
        }
        if rest.starts_with("above") {
            return Some(Marker::Add(Placement::Above));
        }
        if rest.starts_with("below") {
            return Some(Marker::Add(Placement::Below));
        }
        if rest.starts_with("inside") {
            return Some(Marker::Add(Placement::Inside));
        }
        if rest.contains("end of this file") || rest.contains("end of the file") {
            return None; // wird vom Aufrufer als eigener AppendToEnd-Fall behandelt
        }
        if rest.starts_with("at the end") || rest == "at end" {
            return Some(Marker::Add(Placement::AtEnd));
        }
        // Unbekannter Zusatztext (z.B. "in MODULES_DICT and set an ID for
        // you") - kein zuverlässiger Automatismus möglich.
        return Some(Marker::Freeform(trimmed.to_string()));
    }
    None
}

fn looks_like_append_to_end(line: &str) -> bool {
    let trimmed = line.trim();
    let stripped = trimmed
        .strip_prefix("##")
        .or_else(|| trimmed.strip_prefix("//"))
        .or_else(|| trimmed.strip_prefix('#'));
    let Some(body) = stripped else { return false };
    let lower = body.to_lowercase();
    lower.contains("add") && (lower.contains("end of this file") || lower.contains("end of the file"))
}

fn join_lines(lines: &[&str]) -> String {
    // Führende/nachfolgende Leerzeilen im erfassten Block sind reine
    // Formatierung der Paket-Datei, kein Teil des eigentlichen Ankers/Codes.
    let mut start = 0;
    let mut end = lines.len();
    while start < end && lines[start].trim().is_empty() {
        start += 1;
    }
    while end > start && lines[end - 1].trim().is_empty() {
        end -= 1;
    }
    lines[start..end].join("\n")
}

#[derive(PartialEq)]
enum State {
    Idle,
    CapturingScope { start: usize },
    CapturingAnchor { start: usize },
    /// `anchor` beginnt mit `"__freeform__{index}"`, wenn dieser
    /// Code-Block eigentlich zu einer bereits in `ops` liegenden
    /// `FreeformInstruction` gehört (deren `code` erst hier nachträglich
    /// befüllt wird), statt eine echte `SearchInsert`-Anker-Zeichenkette
    /// zu sein.
    CapturingCode { anchor: String, placement: Placement, start: usize },
}

/// Schließt den aktuell offenen Erfassungsblock ab (Ende ist entweder ein
/// neuer Marker oder das Dateiende) und trägt das Ergebnis in `ops` bzw.
/// `current_scope` ein. Läuft für **jede** State/Marker-Kombination
/// gleich, damit z.B. eine Freitext-Anweisung direkt nach Dateibeginn
/// (State::Idle) genauso funktioniert wie eine nach einem search-Block.
fn finalize_state(
    state: State,
    end: usize,
    lines: &[&str],
    ops: &mut Vec<PatchOp>,
    current_scope: &mut Option<String>,
) {
    match state {
        State::CapturingCode { anchor, placement, start } => {
            let code = join_lines(&lines[start..end]);
            if let Some(idx_str) = anchor.strip_prefix("__freeform__") {
                if let Ok(idx) = idx_str.parse::<usize>() {
                    if let Some(PatchOp::FreeformInstruction { code: c, .. }) = ops.get_mut(idx) {
                        *c = code;
                    }
                }
            } else if anchor == "__append_to_end__" {
                ops.push(PatchOp::AppendToEnd { code });
            } else {
                ops.push(PatchOp::SearchInsert { scope: current_scope.clone(), anchor, placement, code });
            }
        }
        State::CapturingScope { start } => {
            *current_scope = Some(join_lines(&lines[start..end]));
        }
        State::CapturingAnchor { .. } | State::Idle => {}
    }
}

pub fn parse_system_file(content: &str) -> ParsedSystemFile {
    let lines: Vec<&str> = content.lines().collect();

    // Kein einziger Marker im ganzen Text? Dann entweder ein reiner
    // "add on the end of this file"-Kopf ohne search/add-Struktur, oder
    // eine echte Ganzdatei (neue Datei) ohne jede Anweisung.
    let has_any_marker = lines.iter().any(|l| classify_marker_line(l).is_some());
    if !has_any_marker {
        if let Some(first_non_blank) = lines.iter().position(|l| !l.trim().is_empty()) {
            if looks_like_append_to_end(lines[first_non_blank]) {
                let code = join_lines(&lines[first_non_blank + 1..]);
                return ParsedSystemFile {
                    ops: vec![PatchOp::AppendToEnd { code }],
                };
            }
        }
        return ParsedSystemFile { ops: vec![] };
    }

    let mut ops = Vec::new();
    let mut current_scope: Option<String> = None;
    let mut state = State::Idle;

    for (i, line) in lines.iter().enumerate() {
        // "add at the end of the file" kann auch mitten in einer Datei mit
        // sonstigen search/inside-Blöcken auftauchen (nicht nur als einzige
        // Kopfzeile der ganzen Datei, siehe der has_any_marker==false-Fall
        // oben) - classify_marker_line gibt hierfür bewusst None zurück,
        // damit hier ein eigener AppendToEnd-Block eröffnet wird statt in
        // den vorherigen search/add-Block hineingelesen zu werden.
        if classify_marker_line(line).is_none() && looks_like_append_to_end(line) {
            let old = std::mem::replace(
                &mut state,
                State::CapturingCode { anchor: "__append_to_end__".to_string(), placement: Placement::AtEnd, start: i + 1 },
            );
            finalize_state(old, i, &lines, &mut ops, &mut current_scope);
            continue;
        }
        match classify_marker_line(line) {
            None => {}
            Some(Marker::Search) => {
                let old = std::mem::replace(&mut state, State::CapturingAnchor { start: i + 1 });
                finalize_state(old, i, &lines, &mut ops, &mut current_scope);
            }
            Some(Marker::Inside) => {
                let old = std::mem::replace(&mut state, State::CapturingScope { start: i + 1 });
                finalize_state(old, i, &lines, &mut ops, &mut current_scope);
            }
            Some(Marker::Add(placement)) => match &state {
                State::CapturingAnchor { start } => {
                    let anchor = join_lines(&lines[*start..i]);
                    state = State::CapturingCode { anchor, placement, start: i + 1 };
                }
                _ => {
                    // "add" ohne vorherigen search-Block (z.B. "add at the
                    // end" direkt nach einem inside-Marker) - kein eigener
                    // Ankertext nötig.
                    let old = std::mem::replace(
                        &mut state,
                        State::CapturingCode { anchor: String::new(), placement, start: i + 1 },
                    );
                    finalize_state(old, i, &lines, &mut ops, &mut current_scope);
                }
            },
            Some(Marker::Freeform(instruction)) => {
                let old = std::mem::replace(&mut state, State::Idle);
                finalize_state(old, i, &lines, &mut ops, &mut current_scope);
                ops.push(PatchOp::FreeformInstruction { instruction, code: String::new() });
                let idx = ops.len() - 1;
                state = State::CapturingCode {
                    anchor: format!("__freeform__{idx}"),
                    placement: Placement::Below,
                    start: i + 1,
                };
            }
        }
    }

    finalize_state(state, lines.len(), &lines, &mut ops, &mut current_scope);
    ParsedSystemFile { ops }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Alle Fixtures unten sind wortgleiche (nur gekürzte) Auszüge aus den
    // beiden echten Beispielsystemen unter C:\Users\DevSteven\Desktop\Systeme
    // - nicht erfunden.

    #[test]
    fn append_to_end_from_cmd_gm_cpp() {
        let content = "// add on the end of this file\n\n#ifdef ADMINPANEL_MOD_CREATE_ITEM_ASLAN\nACMD(do_adminpanel_create_item)\n{\n}\n#endif\n";
        let parsed = parse_system_file(content);
        assert_eq!(parsed.ops.len(), 1);
        match &parsed.ops[0] {
            PatchOp::AppendToEnd { code } => {
                assert!(code.starts_with("#ifdef ADMINPANEL_MOD_CREATE_ITEM_ASLAN"));
                assert!(code.trim_end().ends_with("#endif"));
            }
            other => panic!("expected AppendToEnd, got {other:?}"),
        }
    }

    #[test]
    fn single_search_add_above_from_item_manager_cpp() {
        let content = "// search\n\nCItemManager::CItemManager() : m_pSelectedItemData(NULL)\n{\n}\n\n// add above\n\n#ifdef ADMINPANEL_MOD_CREATE_ITEM_ASLAN\nCItemManager::TItemMap* CItemManager::GetAllItemVnums()\n{\n\treturn &m_ItemMap;\n}\n#endif\n";
        let parsed = parse_system_file(content);
        assert_eq!(parsed.ops.len(), 1);
        match &parsed.ops[0] {
            PatchOp::SearchInsert { anchor, placement, code, scope } => {
                assert_eq!(anchor, "CItemManager::CItemManager() : m_pSelectedItemData(NULL)\n{\n}");
                assert_eq!(*placement, Placement::Above);
                assert!(code.contains("GetAllItemVnums"));
                assert!(scope.is_none());
            }
            other => panic!("expected SearchInsert, got {other:?}"),
        }
    }

    #[test]
    fn bare_add_defaults_to_below_from_defines_h() {
        let content = "// search\n\n#ifdef ENABLE_ASLAN_MODULAR_ADMIN_PANEL\n\n// add \n\n\t#define ADMINPANEL_MOD_CREATE_ITEM_ASLAN\n";
        let parsed = parse_system_file(content);
        assert_eq!(parsed.ops.len(), 1);
        match &parsed.ops[0] {
            PatchOp::SearchInsert { anchor, placement, code, .. } => {
                assert_eq!(anchor, "#ifdef ENABLE_ASLAN_MODULAR_ADMIN_PANEL");
                assert_eq!(*placement, Placement::Below);
                assert_eq!(code, "\t#define ADMINPANEL_MOD_CREATE_ITEM_ASLAN");
            }
            other => panic!("expected SearchInsert, got {other:?}"),
        }
    }

    #[test]
    fn two_search_add_blocks_in_one_file_from_python_item_module_cpp() {
        let content = "// search\nfirst anchor line\n// add above\nfirst code\n// search\nsecond anchor line\n// add\nsecond code\n";
        let parsed = parse_system_file(content);
        assert_eq!(parsed.ops.len(), 2);
        match &parsed.ops[0] {
            PatchOp::SearchInsert { anchor, placement, code, .. } => {
                assert_eq!(anchor, "first anchor line");
                assert_eq!(*placement, Placement::Above);
                assert_eq!(code, "first code");
            }
            other => panic!("expected SearchInsert, got {other:?}"),
        }
        match &parsed.ops[1] {
            PatchOp::SearchInsert { anchor, placement, code, .. } => {
                assert_eq!(anchor, "second anchor line");
                assert_eq!(*placement, Placement::Below);
                assert_eq!(code, "second code");
            }
            other => panic!("expected SearchInsert, got {other:?}"),
        }
    }

    #[test]
    fn freeform_instruction_from_python_application_module_cpp() {
        let content = "// search and edit like this\n\n#ifdef ENABLE_ASLAN_MODULAR_ADMIN_PANEL\n\tPyModule_AddIntConstant(poModule, \"ENABLE_ASLAN_MODULAR_ADMIN_PANEL\", 1);\n#else\n\tPyModule_AddIntConstant(poModule, \"ENABLE_ASLAN_MODULAR_ADMIN_PANEL\", 0);\n#endif\n";
        let parsed = parse_system_file(content);
        assert_eq!(parsed.ops.len(), 1);
        match &parsed.ops[0] {
            PatchOp::FreeformInstruction { instruction, code } => {
                assert_eq!(instruction, "// search and edit like this");
                assert!(code.contains("PyModule_AddIntConstant"));
            }
            other => panic!("expected FreeformInstruction, got {other:?}"),
        }
    }

    #[test]
    fn freeform_instruction_from_uiadminpanel_py_leading_comment() {
        let content = "# add in MODULES_DICT and set ID for you\n\n\tID : { \"modulename\" : \"aslan_create_item\", },\n";
        let parsed = parse_system_file(content);
        assert_eq!(parsed.ops.len(), 1);
        match &parsed.ops[0] {
            PatchOp::FreeformInstruction { instruction, code } => {
                assert!(instruction.contains("MODULES_DICT"));
                assert!(code.contains("aslan_create_item"));
            }
            other => panic!("expected FreeformInstruction, got {other:?}"),
        }
    }

    #[test]
    fn add_above_not_below_from_grp_device_cpp() {
        let content = "//search:\nint CGraphicDevice::Create(HWND hWnd)\n//add ABOVE, not below:\n#ifdef ENABLE_WINDOW_RESIZE\nvoid CGraphicDevice::SetNewSize(int width, int height)\n{\n}\n#endif\n";
        let parsed = parse_system_file(content);
        assert_eq!(parsed.ops.len(), 1);
        match &parsed.ops[0] {
            PatchOp::SearchInsert { anchor, placement, .. } => {
                assert_eq!(anchor, "int CGraphicDevice::Create(HWND hWnd)");
                assert_eq!(*placement, Placement::Above);
            }
            other => panic!("expected SearchInsert, got {other:?}"),
        }
    }

    #[test]
    fn scoped_inside_blocks_from_game_py() {
        let content = "##search:\nfrom _weakref import proxy\n##add below:\nif app.ENABLE_WINDOW_RESIZE:\n\tfrom resizeexpr import Expr\n\n\n##inside:\n\tdef Open(self):\n##search:\n\t\tself.SetSize(wndMgr.GetScreenWidth(), wndMgr.GetScreenHeight())\n##add below:\n\t\tif app.ENABLE_WINDOW_RESIZE:\n\t\t\tself.SetResizeDic({\"size\" : 1})\n\n\n##add at the end:\n\tif app.ENABLE_WINDOW_RESIZE:\n\t\tdef BINARY_UpdateUI(self):\n\t\t\tpass\n";
        let parsed = parse_system_file(content);
        assert_eq!(parsed.ops.len(), 3);

        match &parsed.ops[0] {
            PatchOp::SearchInsert { anchor, placement, scope, .. } => {
                assert_eq!(anchor, "from _weakref import proxy");
                assert_eq!(*placement, Placement::Below);
                assert!(scope.is_none());
            }
            other => panic!("expected SearchInsert, got {other:?}"),
        }
        match &parsed.ops[1] {
            PatchOp::SearchInsert { anchor, placement, scope, code } => {
                assert_eq!(anchor, "\t\tself.SetSize(wndMgr.GetScreenWidth(), wndMgr.GetScreenHeight())");
                assert_eq!(*placement, Placement::Below);
                assert_eq!(scope.as_deref(), Some("\tdef Open(self):"));
                assert!(code.contains("SetResizeDic"));
            }
            other => panic!("expected SearchInsert, got {other:?}"),
        }
        match &parsed.ops[2] {
            PatchOp::SearchInsert { anchor, placement, scope, code } => {
                assert_eq!(anchor, "");
                assert_eq!(*placement, Placement::AtEnd);
                assert_eq!(scope.as_deref(), Some("\tdef Open(self):"));
                assert!(code.contains("BINARY_UpdateUI"));
            }
            other => panic!("expected SearchInsert, got {other:?}"),
        }
    }

    #[test]
    fn append_to_end_of_file_after_other_scoped_blocks_from_ui_py() {
        // Wortgleicher (nur gekürzter) Ausschnitt aus dem echten
        // ResizeWindow1.2-Paket (Client/pack/root/ui.py) - der
        // "##add at the end of the file:"-Block kommt hier NICHT als
        // einzige Kopfzeile der Datei vor (das deckt schon
        // append_to_end_from_cmd_gm_cpp ab), sondern nach einem bereits
        // abgeschlossenen ##inside/##search/##add below-Block. Vorher
        // landete der komplette Rest der Datei fälschlich als Teil des
        // code-Felds des letzten SearchInsert statt als eigener
        // AppendToEnd-Op.
        let content = "##inside:\n\tdef LoadChildren(self, parent, dicChildren):\n##search:\n\t\t\tparent.Children[Index].SetWindowName(Name)\n##add below:\n\t\t\tif app.ENABLE_WINDOW_RESIZE:\n\t\t\t\tpass\n\n\n##add at the end of the file:\nif app.ENABLE_WINDOW_RESIZE:\n\tdef IsDependOnScreen(expr):\n\t\treturn False\n";
        let parsed = parse_system_file(content);
        assert_eq!(parsed.ops.len(), 2);

        match &parsed.ops[0] {
            PatchOp::SearchInsert { anchor, code, .. } => {
                assert_eq!(anchor, "\t\t\tparent.Children[Index].SetWindowName(Name)");
                assert!(!code.contains("add at the end of the file"));
                assert!(!code.contains("IsDependOnScreen"));
            }
            other => panic!("expected SearchInsert, got {other:?}"),
        }
        match &parsed.ops[1] {
            PatchOp::AppendToEnd { code } => {
                assert!(code.starts_with("if app.ENABLE_WINDOW_RESIZE:"));
                assert!(code.contains("IsDependOnScreen"));
            }
            other => panic!("expected AppendToEnd, got {other:?}"),
        }
    }

    #[test]
    fn file_with_no_markers_at_all_is_a_whole_file_candidate() {
        let content = "class ImageBox(Window):\n\tdef __init__(self):\n\t\tpass\n";
        let parsed = parse_system_file(content);
        assert!(parsed.ops.is_empty());
    }

    mod find_anchor_tests {
        use super::*;

        #[test]
        fn exact_match_when_unique() {
            let haystack = "line one\nCItemManager::CItemManager() : m_pSelectedItemData(NULL)\n{\n}\nline five\n";
            let anchor = "CItemManager::CItemManager() : m_pSelectedItemData(NULL)\n{\n}";
            match find_anchor(haystack, anchor) {
                AnchorMatch::Found { range, confidence } => {
                    assert_eq!(confidence, AnchorConfidence::Exact);
                    assert_eq!(range, LineRange { start_line: 1, end_line: 3 });
                }
                other => panic!("expected Found, got {other:?}"),
            }
        }

        #[test]
        fn whitespace_tolerant_when_indentation_differs() {
            let haystack = "def Open(self):\n    self.SetSize(1, 2)\n"; // Spaces statt Tab
            let anchor = "\tself.SetSize(1, 2)"; // Tab wie im Systempaket
            match find_anchor(haystack, anchor) {
                AnchorMatch::Found { confidence, .. } => {
                    assert_eq!(confidence, AnchorConfidence::WhitespaceTolerant);
                }
                other => panic!("expected Found, got {other:?}"),
            }
        }

        #[test]
        fn comment_stripped_when_comment_wording_differs() {
            let haystack = "int Foo() {\n\t// aktuelle Version 2.1\n\tDoStuff();\n}\n";
            let anchor = "int Foo() {\n\t// alte Version 1.0\n\tDoStuff();\n}";
            match find_anchor(haystack, anchor) {
                AnchorMatch::Found { confidence, .. } => {
                    assert_eq!(confidence, AnchorConfidence::CommentStripped);
                }
                other => panic!("expected Found, got {other:?}"),
            }
        }

        #[test]
        fn partial_match_falls_back_to_last_code_line() {
            let haystack = "totally different context\nDoStuff();\nmore lines\n";
            let anchor = "some preceding line that no longer exists\nDoStuff();";
            match find_anchor(haystack, anchor) {
                AnchorMatch::Found { confidence, range } => {
                    assert_eq!(confidence, AnchorConfidence::Partial);
                    assert_eq!(range, LineRange { start_line: 1, end_line: 1 });
                }
                other => panic!("expected Found (partial), got {other:?}"),
            }
        }

        #[test]
        fn ambiguous_when_anchor_appears_twice() {
            let haystack = "DoStuff();\nsomething else\nDoStuff();\n";
            let anchor = "DoStuff();";
            match find_anchor(haystack, anchor) {
                AnchorMatch::Ambiguous { candidates } => assert_eq!(candidates.len(), 2),
                other => panic!("expected Ambiguous, got {other:?}"),
            }
        }

        #[test]
        fn not_found_when_nothing_matches_at_all() {
            let haystack = "completely unrelated content\nnothing here\n";
            let anchor = "NeverExistedFunction();";
            assert_eq!(find_anchor(haystack, anchor), AnchorMatch::NotFound);
        }
    }

    mod find_anchor_with_scope_tests {
        use super::*;

        // Nachgebaut aus dem echten game.py-Fall: dieselbe Zeile
        // "self.SetSize(100, 100)" kommt wortgleich in zwei Methoden vor -
        // der Scope grenzt auf die richtige ein. find_anchor vergleicht
        // ganze Zeilen, kein Teilstring-Suche, daher hier bewusst zwei
        // identische Zeilen statt nur eines gemeinsamen Präfixes.
        const HAYSTACK: &str = "class Window:\n\tdef __init__(self):\n\t\tself.SetSize(100, 100)\n\n\tdef Open(self):\n\t\tself.SetSize(100, 100)\n\n\tdef Close(self):\n\t\tpass\n";

        #[test]
        fn narrows_search_to_the_right_scope_when_anchor_is_ambiguous_globally() {
            // Ohne Scope ist "self.SetSize(100, 100)" zweimal vorhanden - mehrdeutig.
            match find_anchor(HAYSTACK, "\t\tself.SetSize(100, 100)") {
                AnchorMatch::Ambiguous { candidates } => assert_eq!(candidates.len(), 2),
                other => panic!("expected globally ambiguous match, got {other:?}"),
            }

            // Mit Scope "def Open(self):" ist es eindeutig - Zeile 5 (die
            // Kopie in Open), nicht Zeile 2 (die Kopie in __init__).
            match find_anchor_with_scope(HAYSTACK, Some("\tdef Open(self):"), "\t\tself.SetSize(100, 100)") {
                AnchorMatch::Found { range, .. } => assert_eq!(range.start_line, 5),
                other => panic!("expected Found within scope, got {other:?}"),
            }
        }

        #[test]
        fn stops_the_scope_window_at_the_next_dedented_line() {
            // "pass" in Close(self) liegt hinter der nächsten Methode (def
            // Close), die auf Open(self)'s eigener Einrückungsebene
            // dedented - das Fenster für "inside Open(self)" darf da nicht
            // mehr hineinreichen.
            let result = find_anchor_with_scope(HAYSTACK, Some("\tdef Open(self):"), "\t\tpass");
            assert_eq!(result, AnchorMatch::NotFound);
        }

        #[test]
        fn not_found_when_the_scope_itself_cannot_be_located() {
            let result =
                find_anchor_with_scope(HAYSTACK, Some("\tdef NichtVorhanden(self):"), "\t\tself.SetSize(100, 100)");
            assert_eq!(result, AnchorMatch::NotFound);
        }
    }

    mod resolve_insertion_and_splice_tests {
        use super::*;

        #[test]
        fn resolves_above_to_the_anchor_start_line() {
            let haystack = "a\nb\nc\n";
            match resolve_insertion(haystack, None, "b", Placement::Above) {
                InsertionResolution::Ready { line, .. } => assert_eq!(line, 1),
                other => panic!("expected Ready, got {other:?}"),
            }
        }

        #[test]
        fn resolves_below_to_right_after_the_anchor_end_line() {
            let haystack = "a\nb\nc\n";
            match resolve_insertion(haystack, None, "b", Placement::Below) {
                InsertionResolution::Ready { line, .. } => assert_eq!(line, 2),
                other => panic!("expected Ready, got {other:?}"),
            }
        }

        #[test]
        fn resolves_at_end_without_anchor_to_end_of_file_when_no_scope() {
            let haystack = "a\nb\nc\n";
            match resolve_insertion(haystack, None, "", Placement::AtEnd) {
                InsertionResolution::Ready { line, .. } => assert_eq!(line, 3),
                other => panic!("expected Ready, got {other:?}"),
            }
        }

        #[test]
        fn needs_review_when_anchor_is_ambiguous() {
            let haystack = "x\nx\n";
            match resolve_insertion(haystack, None, "x", Placement::Below) {
                InsertionResolution::NeedsReview { .. } => {}
                other => panic!("expected NeedsReview, got {other:?}"),
            }
        }

        #[test]
        fn splice_inserts_multiline_code_at_the_given_line() {
            let content = "a\nb\nc\n";
            let result = splice_lines(content, 1, "x\ny");
            assert_eq!(result, "a\nx\ny\nb\nc\n");
        }

        #[test]
        fn splice_at_end_of_file_appends() {
            let content = "a\nb\n";
            let result = splice_lines(content, 2, "c");
            assert_eq!(result, "a\nb\nc\n");
        }

        #[test]
        fn full_round_trip_from_a_real_search_add_block() {
            // Simuliert genau den ItemManager.cpp-Fall: Anker gefunden,
            // Code oberhalb eingefügt.
            let real_file = "// header\n\nCItemManager::CItemManager() : m_pSelectedItemData(NULL)\n{\n}\n";
            let anchor = "CItemManager::CItemManager() : m_pSelectedItemData(NULL)\n{\n}";
            let code = "#ifdef ADMINPANEL_MOD_CREATE_ITEM_ASLAN\nint extra;\n#endif";
            let resolution = resolve_insertion(real_file, None, anchor, Placement::Above);
            let InsertionResolution::Ready { line, .. } = resolution else {
                panic!("expected Ready, got {resolution:?}");
            };
            let result = splice_lines(real_file, line, code);
            assert_eq!(
                result,
                "// header\n\n#ifdef ADMINPANEL_MOD_CREATE_ITEM_ASLAN\nint extra;\n#endif\nCItemManager::CItemManager() : m_pSelectedItemData(NULL)\n{\n}\n"
            );
        }
    }

    mod check_structural_balance_tests {
        use super::*;

        #[test]
        fn flags_unbalanced_braces_in_cpp_files() {
            let broken = "void Foo() {\n\tif (true) {\n\t\tDoStuff();\n}\n";
            assert!(check_structural_balance("cmd_gm.cpp", broken).is_some());
        }

        #[test]
        fn accepts_balanced_cpp_content() {
            let ok = "void Foo() {\n\tif (true) {\n\t\tDoStuff();\n\t}\n}\n";
            assert_eq!(check_structural_balance("cmd_gm.cpp", ok), None);
        }

        #[test]
        fn flags_unbalanced_ifdef_endif() {
            let broken = "#ifdef X\nvoid Foo() {}\n";
            assert!(check_structural_balance("Defines.h", broken).is_some());
        }

        #[test]
        fn skips_non_cpp_files_entirely() {
            let broken_looking = "def foo():\n\tif True:\n\t\tpass\n"; // unbalanced parens/braces don't apply to Python
            assert_eq!(check_structural_balance("game.py", broken_looking), None);
        }
    }
}
