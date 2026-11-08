//! "Semi-KI"-Freitext-Assistent für den Mehrschritt-Baukasten: reine
//! Mustererkennung (Schlüsselwörter + Fuzzy-Abgleich gegen die echte
//! Item-/Mob-/NPC-Datenbank), KEINE echte KI. Erkennt aus einer
//! Freitext-Beschreibung einzelne Schritte (dialog/collect/kill/use) und
//! füllt damit dieselbe [`MultiStepFormState`]-Struktur, die der Nutzer
//! danach im Baukasten prüft - nichts wird stillschweigend übernommen.
//!
//! Zwei Schichten: [`build_draft_steps`] ist reine Textverarbeitung,
//! [`resolve_description`] löst Namen über eine injizierte Lookup-Funktion
//! auf (im Dialog ein Wrapper um search_items/search_mobs).
//!
//! Portiert aus questDescriptionParser.ts. Die dortigen regulären Ausdrücke
//! sind hier von Hand nachgebaut (ohne Regex-Crate) - mit denselben
//! JS-Eigenheiten: `\b` kennt nur ASCII-Wortzeichen, `/i` vergleicht
//! zeichenweise ohne Groß-/Kleinschreibung.
//!
//! [`MultiStepFormState`]: super::quest_templates::MultiStepFormState

use super::js_num::{is_js_space, num_str, number};
use super::quest_templates::{QuestStep, StepKind};
use std::future::Future;

// ------------------------------------------------------------ Muster-Bausteine

const KILL_VERBS: &[&str] = &["töte", "tote", "besiege", "besiegt", "erledige", "erledigt", "erlege", "erlegt", "jage", "jagt"];
const COLLECT_VERBS: &[&str] = &["sammle", "sammel", "sammelt", "gib", "abgeben", "bring", "bringe", "bringt", "liefere", "liefert"];
const USE_VERBS: &[&str] = &["benutze", "benutzt", "benutz", "verwende", "verwendet", "nutze", "nutzt"];
const DIALOG_VERBS: &[&str] = &["rede", "redet", "sprich", "sprechen", "unterhalte", "unterhalt", "besuche", "besuch"];
const REWARD_MARKERS: &[&str] = &[
    "belohnung", "bekomme", "bekommst", "bekommt", "bekommen", "erhalte", "erhältst", "erhaelst", "erhält", "erhaelt",
    "erhaltet", "erhalten", "kriege", "kriegst", "kriegt", "kriegen",
];
const CONNECTORS: &[&str] = &["dann", "danach", "anschließend"];

const FILLER_WORDS: &[&str] = &[
    "mit", "den", "die", "das", "dem", "der", "einen", "eine", "ein", "x", "stück", "stücke", "mal", "ab", "für", "dafür",
    "und", "dann", "danach", "anschließend", "man", "es", "gibt",
];

/// JS-`\w` ohne `u`-Flag: nur ASCII.
fn is_word(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// JS-`\b` an Byte-Position `i`.
fn boundary(text: &str, i: usize) -> bool {
    is_word(text[..i].chars().next_back()) != is_word(text[i..].chars().next())
}

fn chars_eq_ci(a: char, b: char) -> bool {
    a == b || {
        let mut ua = a.to_uppercase();
        let mut ub = b.to_uppercase();
        match (ua.next(), ua.next(), ub.next(), ub.next()) {
            (Some(x), None, Some(y), None) => x == y,
            _ => false,
        }
    }
}

/// Beginnt `text` mit `pat` (ohne Groß-/Kleinschreibung)? Liefert die Länge
/// des Treffers in `text` (Bytes).
fn ci_prefix(text: &str, pat: &str) -> Option<usize> {
    let mut it = text.char_indices();
    for p in pat.chars() {
        let (_, c) = it.next()?;
        if !chars_eq_ci(c, p) {
            return None;
        }
    }
    Some(it.next().map(|(i, _)| i).unwrap_or(text.len()))
}

fn positions(text: &str) -> impl Iterator<Item = usize> + '_ {
    text.char_indices().map(|(i, _)| i).chain(std::iter::once(text.len()))
}

/// `/\b(alt1|alt2|...)\b/i` - erster Treffer als Byte-Bereich.
fn find_word(text: &str, alts: &[&str]) -> Option<(usize, usize)> {
    for i in positions(text) {
        if !boundary(text, i) {
            continue;
        }
        for alt in alts {
            if let Some(len) = ci_prefix(&text[i..], alt) {
                if boundary(text, i + len) {
                    return Some((i, i + len));
                }
            }
        }
    }
    None
}

fn skip_ws(text: &str, mut i: usize) -> usize {
    while let Some(c) = text[i..].chars().next() {
        if !is_js_space(c) {
            break;
        }
        i += c.len_utf8();
    }
    i
}

fn digit_run(text: &str, i: usize) -> usize {
    i + text[i..].bytes().take_while(|b| b.is_ascii_digit()).count()
}

/// `/(\d+)\s*yang/i` → (Trefferbereich, Zahl)
fn find_yang(text: &str) -> Option<(usize, usize, f64)> {
    for (i, c) in text.char_indices() {
        if !c.is_ascii_digit() {
            continue;
        }
        let j = digit_run(text, i);
        let k = skip_ws(text, j);
        if let Some(len) = ci_prefix(&text[k..], "yang") {
            return Some((i, k + len, number(&text[i..j])));
        }
    }
    None
}

/// `/\b(\d+)\b(?!\s*yang)/i`
fn find_quantity(text: &str) -> Option<f64> {
    for (i, c) in text.char_indices() {
        if !c.is_ascii_digit() || !boundary(text, i) {
            continue;
        }
        let j = digit_run(text, i);
        if !boundary(text, j) {
            continue;
        }
        let k = skip_ws(text, j);
        if ci_prefix(&text[k..], "yang").is_some() {
            continue;
        }
        return Some(number(&text[i..j]));
    }
    None
}

fn is_name_start(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_ascii_uppercase() || matches!(c, 'Ä' | 'Ö' | 'Ü'))
}

fn letters_end(text: &str, mut i: usize) -> usize {
    while let Some(c) = text[i..].chars().next() {
        if !c.is_alphabetic() {
            break;
        }
        i += c.len_utf8();
    }
    i
}

/// `/\bbei\s+([A-ZÄÖÜ][\p{L}]*(?:\s+[A-ZÄÖÜ][\p{L}]*)*)/u` →
/// (ganzer Treffer, Namensgruppe)
fn find_npc_bei(text: &str) -> Option<(String, String)> {
    for i in positions(text) {
        if !boundary(text, i) || !text[i..].starts_with("bei") {
            continue;
        }
        let ws_start = i + 3;
        let name_start = skip_ws(text, ws_start);
        if name_start == ws_start || !is_name_start(text[name_start..].chars().next()) {
            continue;
        }
        let mut end = letters_end(text, name_start + text[name_start..].chars().next().unwrap().len_utf8());
        loop {
            let next = skip_ws(text, end);
            if next == end || !is_name_start(text[next..].chars().next()) {
                break;
            }
            end = letters_end(text, next + text[next..].chars().next().unwrap().len_utf8());
        }
        return Some((text[i..end].to_string(), text[name_start..end].to_string()));
    }
    None
}

/// `/\balle\s+(\d+)\s+tage?n?\b/i` - "alle 4 Tage" / "alle 4 Tagen".
fn find_repeatable(text: &str) -> Option<f64> {
    for i in positions(text) {
        if !boundary(text, i) {
            continue;
        }
        let Some(len) = ci_prefix(&text[i..], "alle") else { continue };
        let a = i + len;
        let d = skip_ws(text, a);
        if d == a {
            continue;
        }
        let de = digit_run(text, d);
        if de == d {
            continue;
        }
        let t = skip_ws(text, de);
        if t == de {
            continue;
        }
        let Some(tl) = ci_prefix(&text[t..], "tag") else { continue };
        let base = t + tl;
        for (e, n) in [("e", "n"), ("e", ""), ("", "n"), ("", "")] {
            let Some(el) = ci_prefix(&text[base..], e) else { continue };
            let Some(nl) = ci_prefix(&text[base + el..], n) else { continue };
            if boundary(text, base + el + nl) {
                return Some(number(&text[d..de]));
            }
        }
    }
    None
}

/// `/^(?:und\s+)?(?:dann|danach|anschließend)\b\s*/i` - Ende des Treffers.
fn connector_prefix_end(text: &str) -> Option<usize> {
    let mut starts = vec![];
    if let Some(len) = ci_prefix(text, "und") {
        let k = skip_ws(text, len);
        if k > len {
            starts.push(k);
        }
    }
    starts.push(0);
    for p in starts {
        for alt in CONNECTORS {
            if let Some(len) = ci_prefix(&text[p..], alt) {
                if boundary(text, p + len) {
                    return Some(skip_ws(text, p + len));
                }
            }
        }
    }
    None
}

fn strip_connector(text: &str) -> &str {
    match connector_prefix_end(text) {
        Some(end) => &text[end..],
        None => text,
    }
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_space)
}

/// `.replace(/[.,;:!?]+$/, "")`
fn strip_trailing_punct(s: &str) -> &str {
    s.trim_end_matches(['.', ',', ';', ':', '!', '?'])
}

fn strip_fillers(text: &str) -> String {
    let words: Vec<&str> = text
        .split(is_js_space)
        .filter(|w| !w.is_empty() && !FILLER_WORDS.contains(&strip_trailing_punct(&w.to_lowercase())))
        .collect();
    js_trim(&words.join(" ")).to_string()
}

/// `text.replace(match, " ")` - nur der erste Vorkommen.
fn strip_word(text: &str, m: &str) -> String {
    text.replacen(m, " ", 1)
}

fn clean_phrase(s: &str) -> String {
    js_trim(strip_trailing_punct(&strip_fillers(s))).to_string()
}

// ------------------------------------------------------------ Klauseln

/// Ob ein Komma-Teil eine neue Klausel beginnt (statt nur ein Zusatz wie
/// "Hans, dem Schmied") - reine Heuristik.
fn looks_like_new_clause(fragment: &str) -> bool {
    let s = strip_connector(fragment);
    find_word(s, KILL_VERBS).is_some()
        || find_word(s, COLLECT_VERBS).is_some()
        || find_word(s, USE_VERBS).is_some()
        || find_word(s, DIALOG_VERBS).is_some()
        || find_word(s, REWARD_MARKERS).is_some()
        || find_yang(s).is_some()
        || find_repeatable(s).is_some()
}

pub fn split_into_clauses(text: &str) -> Vec<String> {
    let mut clauses = vec![];
    for sentence in text.split(['.', '\n', ';']).map(js_trim).filter(|s| !s.is_empty()) {
        let mut current = String::new();
        for part in sentence.split(',').map(js_trim).filter(|s| !s.is_empty()) {
            if current.is_empty() {
                current = part.to_string();
            } else if looks_like_new_clause(part) {
                clauses.push(std::mem::take(&mut current));
                current = js_trim(strip_connector(part)).to_string();
            } else {
                current = format!("{current}, {part}");
            }
        }
        if !current.is_empty() {
            clauses.push(current);
        }
    }
    clauses
}

#[derive(Clone, Debug, PartialEq)]
pub struct RewardExtract {
    pub item_phrase: Option<String>,
    pub money: Option<f64>,
    pub had_marker: bool,
}

pub fn extract_reward(clause: &str) -> RewardExtract {
    let has_marker = find_word(clause, REWARD_MARKERS).is_some();
    let yang = find_yang(clause);
    let money = yang.map(|y| y.2);
    if !has_marker && money.is_none() {
        return RewardExtract { item_phrase: None, money: None, had_marker: false };
    }
    let mut rest = clause.to_string();
    if let Some((a, b, _)) = yang {
        rest = strip_word(&rest, &clause[a..b]);
    }
    if let Some((a, b)) = find_word(&rest, REWARD_MARKERS) {
        rest = format!("{} {}", &rest[..a], &rest[b..]);
    }
    let rest = clean_phrase(&rest);
    RewardExtract { item_phrase: (!rest.is_empty()).then_some(rest), money, had_marker: has_marker }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClauseParse {
    pub kind: StepKind,
    pub recognized: bool,
    pub is_reward_only: bool,
    pub raw_text: String,
    pub quantity: Option<f64>,
    pub target_phrase: Option<String>,
    pub npc_phrase: Option<String>,
    pub reward_item_phrase: Option<String>,
    pub reward_money: Option<f64>,
}

pub fn classify_clause(clause: &str) -> ClauseParse {
    let reward = extract_reward(clause);
    let found = [
        (KILL_VERBS, StepKind::Kill),
        (COLLECT_VERBS, StepKind::Collect),
        (USE_VERBS, StepKind::Use),
        (DIALOG_VERBS, StepKind::Dialog),
    ]
    .into_iter()
    .find_map(|(verbs, kind)| find_word(clause, verbs).map(|m| (m, kind)));

    let Some(((va, vb), kind)) = found else {
        // Kein Aktions-Verb: reine Belohnungs-Klausel (wird in den vorherigen
        // Schritt gemergt) oder Fallback auf einen Dialog-Schritt mit Rohtext.
        return ClauseParse {
            kind: StepKind::Dialog,
            recognized: false,
            is_reward_only: reward.had_marker || reward.money.is_some(),
            raw_text: clause.to_string(),
            quantity: None,
            target_phrase: None,
            npc_phrase: None,
            reward_item_phrase: reward.item_phrase,
            reward_money: reward.money,
        };
    };
    let verb = &clause[va..vb];

    let quantity = if matches!(kind, StepKind::Collect | StepKind::Kill) { find_quantity(clause) } else { None };
    let (npc_phrase, without_npc) = match find_npc_bei(clause) {
        Some((whole, name)) => (Some(js_trim(&name).to_string()), strip_word(clause, &whole)),
        None => (None, clause.to_string()),
    };

    let mut rest = without_npc.replacen(verb, " ", 1);
    if let Some(q) = quantity {
        rest = rest.replacen(&num_str(q), " ", 1);
    }
    let target = clean_phrase(&rest);
    let target_phrase = (!target.is_empty()).then_some(target);

    let is_dialog = kind == StepKind::Dialog;
    ClauseParse {
        kind,
        recognized: true,
        is_reward_only: false,
        raw_text: clause.to_string(),
        quantity,
        target_phrase: if is_dialog { None } else { target_phrase.clone() },
        npc_phrase: if is_dialog { target_phrase } else { npc_phrase },
        reward_item_phrase: reward.item_phrase,
        reward_money: reward.money,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DraftStepsResult {
    pub drafts: Vec<ClauseParse>,
    pub notes: Vec<String>,
    pub repeatable: bool,
    pub cooldown_days: Option<f64>,
}

pub fn build_draft_steps(text: &str) -> DraftStepsResult {
    let mut drafts: Vec<ClauseParse> = vec![];
    let mut notes: Vec<String> = vec![];
    let mut carried_npc: Option<String> = None;
    let mut repeatable = false;
    let mut cooldown_days = None;

    for clause in split_into_clauses(text) {
        if let Some(days) = find_repeatable(&clause) {
            repeatable = true;
            cooldown_days = Some(days);
            continue;
        }

        let mut parsed = classify_clause(&clause);

        if parsed.is_reward_only {
            if let Some(prev) = drafts.last_mut() {
                if parsed.reward_item_phrase.is_some() {
                    prev.reward_item_phrase = parsed.reward_item_phrase;
                }
                if parsed.reward_money.is_some() {
                    prev.reward_money = parsed.reward_money;
                }
            } else {
                notes.push(format!("Belohnung \"{}\" konnte keinem Schritt zugeordnet werden.", js_trim(&clause)));
            }
            continue;
        }

        if !parsed.recognized {
            notes.push(format!(
                "Schritt {}: keine bekannte Aktion erkannt - als Dialogtext übernommen, bitte prüfen.",
                drafts.len() + 1
            ));
        }

        // "use"-Schritte brauchen keinen NPC; alle anderen übernehmen den
        // zuletzt erkannten NPC, bis ein neuer genannt wird.
        if parsed.kind != StepKind::Use {
            match &parsed.npc_phrase {
                None => parsed.npc_phrase = carried_npc.clone(),
                Some(p) => carried_npc = Some(p.clone()),
            }
            if parsed.npc_phrase.is_none() {
                notes.push(format!("Schritt {}: kein NPC erkannt - bitte manuell auswählen.", drafts.len() + 1));
            }
        }

        if matches!(parsed.kind, StepKind::Collect | StepKind::Kill) && parsed.quantity.is_none() {
            notes.push(format!("Schritt {}: keine Anzahl erkannt, \"1\" übernommen - bitte prüfen.", drafts.len() + 1));
        }

        drafts.push(parsed);
    }

    DraftStepsResult { drafts, notes, repeatable, cooldown_days }
}

// ------------------------------------------------------------ Namensauflösung

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct NameCandidate {
    pub vnum: i64,
    pub name: String,
}

fn js_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// search_items/search_mobs sortieren nicht nach Relevanz - das übernimmt
/// diese Bewertung: exakt > beginnt mit > eindeutig kürzester Teilstring.
/// Mehrdeutig oder kein Treffer → `None` (Feld bleibt leer statt falsch).
pub fn pick_best_match(query: &str, candidates: &[NameCandidate]) -> Option<NameCandidate> {
    if candidates.is_empty() {
        return None;
    }
    let q = js_trim(query).to_lowercase();
    if q.is_empty() {
        return None;
    }
    let norm = |c: &NameCandidate| js_trim(&c.name).to_lowercase();

    let exact: Vec<&NameCandidate> = candidates.iter().filter(|c| norm(c) == q).collect();
    if exact.len() == 1 {
        return Some(exact[0].clone());
    }
    if exact.len() > 1 {
        return None;
    }

    let prefix: Vec<&NameCandidate> = candidates.iter().filter(|c| norm(c).starts_with(&q)).collect();
    if prefix.len() == 1 {
        return Some(prefix[0].clone());
    }
    let substring: Vec<&NameCandidate> = candidates.iter().filter(|c| norm(c).contains(&q)).collect();
    let mut pool = if prefix.len() > 1 { prefix } else { substring };
    if pool.is_empty() {
        return None;
    }
    pool.sort_by_key(|c| js_len(&c.name));
    if pool.len() > 1 && js_len(&pool[0].name) == js_len(&pool[1].name) {
        return None;
    }
    Some(pool[0].clone())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LookupKind {
    Npc,
    Item,
    Mob,
}

/// Welches QuestStep-Feld ein fehlgeschlagener Lookup betrifft - die UI
/// bietet dafür direkt einen "Suchen"-Knopf an.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnresolvedField {
    NpcVnum,
    RequiredItemVnum,
    UseItemVnum,
    MobVnum,
    RewardItemVnum,
}

impl UnresolvedField {
    pub fn set(self, step: &mut QuestStep, value: String) {
        match self {
            UnresolvedField::NpcVnum => step.npc_vnum = value,
            UnresolvedField::RequiredItemVnum => step.required_item_vnum = value,
            UnresolvedField::UseItemVnum => step.use_item_vnum = value,
            UnresolvedField::MobVnum => step.mob_vnum = value,
            UnresolvedField::RewardItemVnum => step.reward_item_vnum = value,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnresolvedLookup {
    pub step_index: usize,
    pub field: UnresolvedField,
    pub kind: LookupKind,
    pub query: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolveResult {
    pub steps: Vec<QuestStep>,
    pub notes: Vec<String>,
    pub unresolved: Vec<UnresolvedLookup>,
    pub repeatable: bool,
    pub cooldown_days: Option<f64>,
}

/// Löst die erkannten Namen über `lookup` auf. Ein Fehler des Lookups
/// (z.B. fehlgeschlagenes `invoke`) bricht wie in der TS-Fassung die ganze
/// Analyse ab.
pub async fn resolve_description<F, Fut>(text: &str, lookup: F) -> Result<ResolveResult, String>
where
    F: Fn(String, LookupKind) -> Fut,
    Fut: Future<Output = Result<Option<NameCandidate>, String>>,
{
    let DraftStepsResult { drafts, notes, repeatable, cooldown_days } = build_draft_steps(text);
    let mut all_notes = notes;
    let mut unresolved = vec![];
    let mut steps = vec![];

    for (index, draft) in drafts.into_iter().enumerate() {
        let mut step = QuestStep { kind: draft.kind, ..QuestStep::default() };

        macro_rules! resolve {
            ($phrase:expr, $kind:expr, $field:expr, $label:expr) => {{
                let phrase: String = $phrase;
                match lookup(phrase.clone(), $kind).await? {
                    Some(m) => Some(m),
                    None => {
                        all_notes.push(format!("Schritt {}: {} \"{}\" nicht eindeutig gefunden.", index + 1, $label, phrase));
                        unresolved.push(UnresolvedLookup { step_index: index, field: $field, kind: $kind, query: phrase });
                        None
                    }
                }
            }};
        }

        if draft.kind == StepKind::Dialog {
            step.dialog_text = draft.raw_text.clone();
        }
        if let Some(npc) = draft.npc_phrase.clone() {
            if let Some(m) = resolve!(npc, LookupKind::Npc, UnresolvedField::NpcVnum, "NPC") {
                step.npc_vnum = m.vnum.to_string();
            }
        }
        match draft.kind {
            StepKind::Collect => {
                step.required_item_count = num_str(draft.quantity.unwrap_or(1.0));
                if let Some(t) = draft.target_phrase.clone() {
                    if let Some(m) = resolve!(t, LookupKind::Item, UnresolvedField::RequiredItemVnum, "Item") {
                        step.required_item_vnum = m.vnum.to_string();
                    }
                }
            }
            StepKind::Use => {
                if let Some(t) = draft.target_phrase.clone() {
                    if let Some(m) = resolve!(t, LookupKind::Item, UnresolvedField::UseItemVnum, "Item") {
                        step.use_item_vnum = m.vnum.to_string();
                    }
                }
            }
            StepKind::Kill => {
                step.required_kills = num_str(draft.quantity.unwrap_or(1.0));
                if let Some(t) = draft.target_phrase.clone() {
                    if let Some(m) = resolve!(t, LookupKind::Mob, UnresolvedField::MobVnum, "Monster") {
                        step.mob_vnum = m.vnum.to_string();
                    }
                }
            }
            StepKind::Dialog => {}
        }

        if let Some(money) = draft.reward_money {
            step.reward_money = num_str(money);
        }
        if let Some(r) = draft.reward_item_phrase.clone() {
            if let Some(m) = resolve!(r, LookupKind::Item, UnresolvedField::RewardItemVnum, "Belohnungs-Item") {
                step.reward_item_vnum = m.vnum.to_string();
            }
        }
        steps.push(step);
    }

    Ok(ResolveResult { steps, notes: all_notes, unresolved, repeatable, cooldown_days })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};

    fn block_on<F: Future>(f: F) -> F::Output {
        let mut f = pin!(f);
        let mut cx = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
                return v;
            }
        }
    }

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn some(s: &str) -> Option<String> {
        Some(s.to_string())
    }

    async fn none_lookup(_: String, _: LookupKind) -> Result<Option<NameCandidate>, String> {
        Ok(None)
    }

    #[test]
    fn splits_on_hard_punctuation() {
        assert_eq!(
            split_into_clauses("Rede mit Hans. Sammle Items\nTöte den Boss; fertig"),
            v(&["Rede mit Hans", "Sammle Items", "Töte den Boss", "fertig"])
        );
    }

    #[test]
    fn splits_on_connectors() {
        assert_eq!(
            split_into_clauses("Sammle 10 Wolfsfelle, dann töte 5 Wölfe, danach rede mit Hans"),
            v(&["Sammle 10 Wolfsfelle", "töte 5 Wölfe", "rede mit Hans"])
        );
    }

    #[test]
    fn splits_on_bare_comma_before_new_action() {
        assert_eq!(
            split_into_clauses("Rede mit Hans, sammle 10 Wolfsfelle, dann bekommt man 100 Yang."),
            v(&["Rede mit Hans", "sammle 10 Wolfsfelle", "bekommt man 100 Yang"])
        );
    }

    #[test]
    fn no_split_without_connector() {
        assert_eq!(split_into_clauses("Rede mit Hans, dem Schmied"), v(&["Rede mit Hans, dem Schmied"]));
    }

    #[test]
    fn classify_dialog() {
        let r = classify_clause("Rede mit Hans");
        assert_eq!(r.kind, StepKind::Dialog);
        assert!(r.recognized);
        assert_eq!(r.npc_phrase, some("Hans"));
    }

    #[test]
    fn classify_collect() {
        let r = classify_clause("Sammle 10 Wolfsfelle");
        assert_eq!(r.kind, StepKind::Collect);
        assert_eq!(r.quantity, Some(10.0));
        assert_eq!(r.target_phrase, some("Wolfsfelle"));
        assert_eq!(r.npc_phrase, None);
    }

    #[test]
    fn classify_kill() {
        let r = classify_clause("töte 5 Wölfe");
        assert_eq!(r.kind, StepKind::Kill);
        assert_eq!(r.quantity, Some(5.0));
        assert_eq!(r.target_phrase, some("Wölfe"));
    }

    #[test]
    fn classify_use() {
        let r = classify_clause("benutze den Trank");
        assert_eq!(r.kind, StepKind::Use);
        assert_eq!(r.target_phrase, some("Trank"));
        assert_eq!(r.npc_phrase, None);
    }

    #[test]
    fn classify_bei_npc() {
        let r = classify_clause("gib die Felle bei Hans ab");
        assert_eq!(r.kind, StepKind::Collect);
        assert_eq!(r.npc_phrase, some("Hans"));
        assert_eq!(r.target_phrase, some("Felle"));
    }

    #[test]
    fn classify_fallback() {
        let r = classify_clause("Das ist eine seltsame Formulierung ohne Verb");
        assert_eq!(r.kind, StepKind::Dialog);
        assert!(!r.recognized);
        assert!(!r.is_reward_only);
        assert_eq!(r.raw_text, "Das ist eine seltsame Formulierung ohne Verb");
    }

    #[test]
    fn classify_reward_only() {
        let r = classify_clause("Man bekommt 100 Yang und ein Schwert");
        assert!(!r.recognized);
        assert!(r.is_reward_only);
        assert_eq!(r.reward_money, Some(100.0));
        assert_eq!(r.reward_item_phrase, some("Schwert"));
    }

    #[test]
    fn classify_reward_with_action() {
        let r = classify_clause("töte den Boss und bekomme dafür 100 Yang");
        assert_eq!(r.kind, StepKind::Kill);
        assert!(r.recognized);
        assert!(!r.is_reward_only);
        assert_eq!(r.reward_money, Some(100.0));
    }

    #[test]
    fn merges_trailing_reward() {
        let r = build_draft_steps("Rede mit Hans. Sammle 10 Wolfsfelle, dann töte 5 Wölfe. Man bekommt 100 Yang und ein Schwert.");
        assert_eq!(r.drafts.len(), 3);
        let last = r.drafts.last().unwrap();
        assert_eq!(last.kind, StepKind::Kill);
        assert_eq!(last.reward_money, Some(100.0));
        assert_eq!(last.reward_item_phrase, some("Schwert"));
        assert!(!r.notes.iter().any(|n| n.contains("keinem Schritt zugeordnet")));
    }

    #[test]
    fn carries_npc_forward() {
        let r = build_draft_steps("Rede mit Hans. Sammle 10 Wolfsfelle.");
        assert_eq!(r.drafts[0].npc_phrase, some("Hans"));
        assert_eq!(r.drafts[1].npc_phrase, some("Hans"));
    }

    #[test]
    fn use_needs_no_npc() {
        let r = build_draft_steps("benutze den Trank");
        assert_eq!(r.drafts[0].npc_phrase, None);
        assert!(!r.notes.iter().any(|n| n.contains("kein NPC erkannt")));
    }

    #[test]
    fn flags_missing_npc() {
        let r = build_draft_steps("Sammle 10 Wolfsfelle.");
        assert!(r.notes.iter().any(|n| n.contains("kein NPC erkannt")));
    }

    #[test]
    fn flags_missing_quantity() {
        let r = build_draft_steps("Rede mit Hans. Sammle Wolfsfelle.");
        assert!(r.notes.iter().any(|n| n.contains("keine Anzahl erkannt")));
    }

    #[test]
    fn unassignable_reward() {
        let r = build_draft_steps("Man bekommt 100 Yang.");
        assert!(r.drafts.is_empty());
        assert!(r.notes.iter().any(|n| n.contains("keinem Schritt zugeordnet")));
    }

    fn cand(vnum: i64, name: &str) -> NameCandidate {
        NameCandidate { vnum, name: name.to_string() }
    }

    #[test]
    fn pick_best_match_cases() {
        let c = vec![cand(1, "Wolfsfell"), cand(2, "Wolfsfellstück"), cand(3, "Zerrissenes Wolfsfell")];
        assert_eq!(pick_best_match("wolfsfell", &c), Some(cand(1, "Wolfsfell")));
        let no_exact = vec![cand(2, "Wolfsfellstück"), cand(3, "Zerrissenes Wolfsfell")];
        assert_eq!(pick_best_match("wolfsfell", &no_exact), Some(cand(2, "Wolfsfellstück")));
        assert_eq!(pick_best_match("drachenschuppe", &c), None);
        let tie = vec![cand(4, "AB Fell"), cand(5, "CD Fell")];
        assert_eq!(pick_best_match("fell", &tie), None);
        assert_eq!(pick_best_match("", &c), None);
    }

    #[test]
    fn resolve_fills_fields() {
        let lookup = |phrase: String, kind: LookupKind| async move {
            Ok(match (kind, phrase.as_str()) {
                (LookupKind::Npc, "Hans") => Some(cand(9001, "Hans")),
                (LookupKind::Item, "Wolfsfelle") => Some(cand(500, "Wolfsfell")),
                _ => None,
            })
        };
        let r = block_on(resolve_description("Rede mit Hans. Sammle 10 Wolfsfelle.", lookup)).unwrap();
        assert_eq!(r.steps.len(), 2);
        assert_eq!(r.steps[0].kind, StepKind::Dialog);
        assert_eq!(r.steps[0].npc_vnum, "9001");
        assert_eq!(r.steps[1].kind, StepKind::Collect);
        assert_eq!(r.steps[1].npc_vnum, "9001");
        assert_eq!(r.steps[1].required_item_vnum, "500");
        assert_eq!(r.steps[1].required_item_count, "10");
        assert!(r.notes.is_empty());
    }

    #[test]
    fn resolve_leaves_field_empty_with_note() {
        let r = block_on(resolve_description("Sammle 10 Wolfsfelle bei Hans.", none_lookup)).unwrap();
        assert_eq!(r.steps[0].npc_vnum, "");
        assert_eq!(r.steps[0].required_item_vnum, "");
        assert!(r.notes.iter().any(|n| n.contains("NPC \"Hans\"")));
        assert!(r.notes.iter().any(|n| n.contains("Item \"Wolfsfelle\"")));
    }

    #[test]
    fn resolve_structured_unresolved() {
        let r = block_on(resolve_description("Sammle 10 Wolfsfelle bei Hans.", none_lookup)).unwrap();
        assert!(r.unresolved.contains(&UnresolvedLookup {
            step_index: 0,
            field: UnresolvedField::NpcVnum,
            kind: LookupKind::Npc,
            query: "Hans".into()
        }));
        assert!(r.unresolved.contains(&UnresolvedLookup {
            step_index: 0,
            field: UnresolvedField::RequiredItemVnum,
            kind: LookupKind::Item,
            query: "Wolfsfelle".into()
        }));
    }

    #[test]
    fn resolve_use_item_field() {
        let r = block_on(resolve_description("benutze den Trank", none_lookup)).unwrap();
        assert!(r.unresolved.contains(&UnresolvedLookup {
            step_index: 0,
            field: UnresolvedField::UseItemVnum,
            kind: LookupKind::Item,
            query: "Trank".into()
        }));
    }

    #[test]
    fn resolve_carries_repeatable() {
        let r = block_on(resolve_description("Rede mit Hans, alle 4 Tage wiederholbar.", none_lookup)).unwrap();
        assert!(r.repeatable);
        assert_eq!(r.cooldown_days, Some(4.0));
    }

    #[test]
    fn reward_marker_coverage() {
        for clause in [
            "Man erhalte 100 Yang",
            "Man erhältst 100 Yang",
            "Man erhaltet 100 Yang",
            "Man bekomme 100 Yang",
            "Man bekommst 100 Yang",
            "Man kriege 100 Yang",
            "Man kriegst 100 Yang",
            "Man kriegt 100 Yang",
        ] {
            let r = classify_clause(clause);
            assert!(r.is_reward_only, "{clause}");
            assert_eq!(r.reward_money, Some(100.0), "{clause}");
        }
    }

    #[test]
    fn repeatable_own_clause() {
        assert_eq!(
            split_into_clauses("Sammle 10 Felle, du kannst das alle 4 Tage machen"),
            v(&["Sammle 10 Felle", "du kannst das alle 4 Tage machen"])
        );
    }

    #[test]
    fn repeatable_not_a_step() {
        let r = build_draft_steps("Rede mit Hans. Du kannst diese Quest alle 4 Tage machen.");
        assert_eq!(r.drafts.len(), 1);
        assert!(r.repeatable);
        assert_eq!(r.cooldown_days, Some(4.0));
    }

    #[test]
    fn repeatable_tagen() {
        let r = build_draft_steps("Wiederholbar alle 7 Tagen.");
        assert!(r.repeatable);
        assert_eq!(r.cooldown_days, Some(7.0));
    }

    #[test]
    fn bei_separates_npc_from_item() {
        let r = build_draft_steps("gib 20 Orkzähne bei Gemi ab, dann bekommst du 100 Yang");
        assert_eq!(r.drafts.len(), 1);
        assert_eq!(r.drafts[0].kind, StepKind::Collect);
        assert_eq!(r.drafts[0].quantity, Some(20.0));
        assert_eq!(r.drafts[0].target_phrase, some("Orkzähne"));
        assert_eq!(r.drafts[0].npc_phrase, some("Gemi"));
        assert_eq!(r.drafts[0].reward_money, Some(100.0));
    }

    const SENTENCE: &str = "Rede mit dem Gemi, bringe der Gemi, 20 Orkzähne und erhalte dafür 250HP zusätzlich dauerhaft, du kannst diese quest alle 4 Tage machen";

    #[test]
    fn regression_clean_clauses() {
        assert_eq!(
            split_into_clauses(SENTENCE),
            v(&[
                "Rede mit dem Gemi",
                "bringe der Gemi",
                "20 Orkzähne und erhalte dafür 250HP zusätzlich dauerhaft",
                "du kannst diese quest alle 4 Tage machen",
            ])
        );
    }

    #[test]
    fn regression_repeatable() {
        let r = build_draft_steps(SENTENCE);
        assert!(r.repeatable);
        assert_eq!(r.cooldown_days, Some(4.0));
        assert_eq!(r.drafts.len(), 2);
    }

    #[test]
    fn regression_first_npc_resolves() {
        let lookup = |phrase: String, kind: LookupKind| async move {
            Ok((kind == LookupKind::Npc && phrase == "Gemi").then(|| cand(9042, "Gemischwarenhändlerin")))
        };
        let r = block_on(resolve_description(SENTENCE, lookup)).unwrap();
        assert_eq!(r.steps[0].kind, StepKind::Dialog);
        assert_eq!(r.steps[0].npc_vnum, "9042");
    }
}
