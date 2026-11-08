//! Lua-Generatoren für den "Vorlagen"-Modus des Quest Builders. Jedes hier
//! verwendete Konstrukt (when NPC.chat."label" [with cond] begin...end, when
//! MOB.kill begin...end, when ITEM.use begin...end, say/say_title,
//! pc.give_item2, pc.changemoney, pc.count_item/remove_item, pc.getqf/setqf,
//! string.format) wurde gegen echte Quest-Dateien des Dev-Servers geprüft
//! (Pferd/Pferd_rufen.lua, Biologie/Orkzahn.lua). Die Vorlagen decken nur die
//! üblichen Fälle ab; alles Komplexere wird nach dem Anlegen als Code
//! bearbeitet. (Portiert aus questTemplates.ts - Ausgabe zeichengenau gleich.)

use super::js_num::{num_str, number, number_or, number_or0};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum TemplateType {
    Dialog,
    Collect,
    ChanceCollect,
    Kill,
    Use,
    BuffedItem,
    Dungeon,
    MultiStep,
}

impl TemplateType {
    /// Reihenfolge wie `Object.entries(TEMPLATE_LABELS)`.
    pub const ALL: [TemplateType; 8] = [
        TemplateType::Dialog,
        TemplateType::Collect,
        TemplateType::ChanceCollect,
        TemplateType::Kill,
        TemplateType::Use,
        TemplateType::BuffedItem,
        TemplateType::Dungeon,
        TemplateType::MultiStep,
    ];

    pub fn id(self) -> &'static str {
        match self {
            TemplateType::Dialog => "dialog",
            TemplateType::Collect => "collect",
            TemplateType::ChanceCollect => "chance_collect",
            TemplateType::Kill => "kill",
            TemplateType::Use => "use",
            TemplateType::BuffedItem => "buffed_item",
            TemplateType::Dungeon => "dungeon",
            TemplateType::MultiStep => "multi_step",
        }
    }

    pub fn from_id(id: &str) -> Option<TemplateType> {
        Self::ALL.iter().copied().find(|t| t.id() == id)
    }

    pub fn label(self) -> &'static str {
        match self {
            TemplateType::Dialog => "NPC-Dialog mit Belohnung",
            TemplateType::Collect => "Sammel-Quest (Items abgeben)",
            TemplateType::ChanceCollect => "Sammel-Quest mit Erfolgschance (Biologie-Stil)",
            TemplateType::Kill => "Kill-Quest (Monster töten)",
            TemplateType::Use => "Item-Benutzung",
            TemplateType::BuffedItem => "Item mit festen Boni verschenken",
            TemplateType::Dungeon => "Run / Instanz-Dungeon (mehrere Etagen)",
            TemplateType::MultiStep => "Mehrschritt-Quest (mehrere Aufgaben nacheinander)",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            TemplateType::Dialog => "Spieler spricht den NPC an und bekommt direkt eine Belohnung.",
            TemplateType::Collect => "Spieler muss eine bestimmte Anzahl eines Items besitzen und beim NPC abgeben, um die Belohnung zu erhalten.",
            TemplateType::ChanceCollect => "Wie bei den echten Biologie-Quests (z.B. Orkzahn.lua): Spieler gibt bei jedem Versuch ein Item ab, das Item wird IMMER verbraucht, zählt aber nur mit einer festgelegten Chance zum Fortschritt. Erst nach genug Erfolgen gibt es die Belohnung.",
            TemplateType::Kill => "Spieler muss eine bestimmte Anzahl eines Monsters töten (Fortschritt wird pro Spieler gespeichert), dann beim NPC melden.",
            TemplateType::Use => "Ein Item löst beim Benutzen direkt einen Effekt/eine Belohnung aus (kein NPC nötig).",
            TemplateType::BuffedItem => "Für vorgegebene Boni (z.B. \"Schwert+9 mit INT 500, STR 700\"), die über die Kisten-Loot-Tabelle nicht möglich sind (die kennt nur VNUM+Anzahl+Chance, keine Attribut-Werte). Spieler spricht den NPC an und bekommt ein konkretes Item (z.B. die bereits im Aufwertungs-Editor angelegte \"+9\"-Stufe) mit bis zu 4 fest eingestellten Bonus-Attributen - keine Server-Änderung nötig, item.set_value ist bereits im Quest-Lua vorhanden.",
            TemplateType::Dungeon => "Grundgerüst für einen mehrstufigen Run wie Beran/Daemonenturm/Teufelskatakomben: NPC-Einstieg, pro Etage ein Boss-Kill schaltet die nächste Etage frei (d.regen_file/d.jump_all/d.setf), am Ende Belohnung + Rauswurf. Deckt bewusst nur das gemeinsame Grundmuster ab - Passwort-Systeme, Zeitlimits o.ä. wie beim echten Beran müssen danach im Code-Editor von Hand ergänzt werden.",
            TemplateType::MultiStep => "Verkettet beliebig viele der bekannten Bausteine (Dialog, Sammeln, Töten, Item benutzen) zu einer echten Mehrschritt-Quest, ganz ohne Code - z.B. \"Rede mit NPC\" → \"sammle 10 Wolfsfelle\" → \"töte den Boss\" → \"rede nochmal für die Belohnung\". Jeder Schritt wird ein eigener state-Block, der Übergang läuft über set_state() - gegen den echten Server-Quellcode verifiziert (questlua_quest.cpp/questpc.cpp), nicht nur aus dem Wiki übernommen.",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuestFormState {
    pub npc_vnum: String,
    pub chat_label: String,
    pub npc_title: String,
    pub dialog_text: String,
    pub reward_item_vnum: String,
    pub reward_item_count: String,
    pub reward_money: String,
    pub required_item_vnum: String,
    pub required_item_count: String,
    pub success_text: String,
    pub fail_text: String,
    pub mob_vnum: String,
    pub required_kills: String,
    pub progress_text: String,
    pub use_item_vnum: String,
    pub use_text: String,
    // nur chance_collect ("Biologie"-Stil)
    pub items_per_attempt: String,
    pub accept_chance_percent: String,
    pub required_successes: String,
    pub no_item_text: String,
    pub complete_text: String,
    // nur buffed_item - Item mit fest gesetzten Bonus-Attributen
    // (item.set_value(index, apply_type, value)); "0" = Slot leer.
    pub buffed_item_vnum: String,
    pub buffed_item_count: String,
    pub buffed_attr_type: [String; 4],
    pub buffed_attr_value: [String; 4],
}

fn s(v: &str) -> String {
    v.to_string()
}

fn zeros() -> [String; 4] {
    ["0".into(), "0".into(), "0".into(), "0".into()]
}

impl Default for QuestFormState {
    fn default() -> Self {
        Self {
            npc_vnum: s(""),
            chat_label: s("Quest"),
            npc_title: s(""),
            dialog_text: s(""),
            reward_item_vnum: s(""),
            reward_item_count: s("1"),
            reward_money: s(""),
            required_item_vnum: s(""),
            required_item_count: s("1"),
            success_text: s(""),
            fail_text: s("Das reicht noch nicht."),
            mob_vnum: s(""),
            required_kills: s("10"),
            progress_text: s("Du hast %d von %d erledigt."),
            use_item_vnum: s(""),
            use_text: s(""),
            items_per_attempt: s("1"),
            accept_chance_percent: s("60"),
            required_successes: s("10"),
            no_item_text: s("Du hast das benötigte Item nicht dabei."),
            complete_text: s("Damit hast du genug gesammelt!"),
            buffed_item_vnum: s(""),
            buffed_item_count: s("1"),
            buffed_attr_type: zeros(),
            buffed_attr_value: zeros(),
        }
    }
}

/// say()/say_title() nehmen einen Lua-String in doppelten Anführungszeichen -
/// was ihn sprengen würde wird escaped, Zeilenumbrüche werden zum
/// spieleigenen "[ENTER]".
pub fn lua_string(text: &str) -> String {
    let escaped = text.replace('\\', "\\\\").replace('"', "\\\"").replace("\r\n", "[ENTER]").replace('\n', "[ENTER]");
    format!("\"{escaped}\"")
}

fn indent<S: AsRef<str>>(lines: &[S], depth: usize) -> String {
    let prefix = "\t".repeat(depth);
    lines
        .iter()
        .map(|l| {
            let l = l.as_ref();
            if l.is_empty() {
                String::new()
            } else {
                format!("{prefix}{l}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// `Math.max(lo, Number(text) || fallback)`
fn max_or(lo: f64, text: &str, fallback: f64) -> f64 {
    lo.max(number_or(text, fallback))
}

fn money_line(text: &str) -> Option<String> {
    let money = number(text);
    (money.is_finite() && money != 0.0).then(|| format!("pc.changemoney({})", num_str(money)))
}

fn reward_lines(form: &QuestFormState) -> Vec<String> {
    let mut lines = vec![];
    let item_vnum = number(&form.reward_item_vnum);
    if item_vnum.is_finite() && item_vnum > 0.0 {
        let count = max_or(1.0, &form.reward_item_count, 1.0);
        lines.push(format!("pc.give_item2({}, {})", num_str(item_vnum), num_str(count)));
    }
    lines.extend(money_line(&form.reward_money));
    lines
}

fn chat_trigger(npc: f64, label: &str) -> String {
    format!("{}.chat.{}", num_str(npc), lua_string(label))
}

fn dialog_template(form: &QuestFormState) -> String {
    let npc = number_or0(&form.npc_vnum);
    let mut body = vec![format!("say_title({})", lua_string(&form.npc_title)), format!("say({})", lua_string(&form.dialog_text))];
    body.extend(reward_lines(form));
    [format!("when {} begin", chat_trigger(npc, &form.chat_label)), indent(&body, 1), s("end")].join("\n")
}

fn collect_template(form: &QuestFormState) -> String {
    let npc = number_or0(&form.npc_vnum);
    let required_vnum = num_str(number_or0(&form.required_item_vnum));
    let required_count = num_str(max_or(1.0, &form.required_item_count, 1.0));
    let mut success = vec![
        format!("say_title({})", lua_string(&form.npc_title)),
        format!("say({})", lua_string(&form.success_text)),
        format!("pc.remove_item({required_vnum}, {required_count})"),
    ];
    success.extend(reward_lines(form));
    let fail = [format!("say_title({})", lua_string(&form.npc_title)), format!("say({})", lua_string(&form.fail_text))];
    [
        format!("when {} begin", chat_trigger(npc, &form.chat_label)),
        format!("\tif pc.count_item({required_vnum}) >= {required_count} then"),
        indent(&success, 2),
        s("\telse"),
        indent(&fail, 2),
        s("\tend"),
        s("end"),
    ]
    .join("\n")
}

/// "Biologie"-Sammel-Quest: jeder Versuch verbraucht das Item und würfelt
/// eine %-Chance, ob er zählt (wie Biologie/Orkzahn.lua auf dem Server).
fn chance_collect_template(form: &QuestFormState) -> String {
    let npc = number_or0(&form.npc_vnum);
    let required_vnum = num_str(number_or0(&form.required_item_vnum));
    let per_attempt = num_str(max_or(1.0, &form.items_per_attempt, 1.0));
    let chance = num_str(100f64.min(0f64.max(number_or0(&form.accept_chance_percent))));
    let required_successes = num_str(max_or(1.0, &form.required_successes, 1.0));
    let title = format!("say_title({})", lua_string(&form.npc_title));

    let mut complete = vec![title.clone(), format!("say({})", lua_string(&form.complete_text))];
    complete.extend(reward_lines(form));
    complete.push(s("pc.setqf(\"collect_count\", 0)"));
    complete.push(s("return"));
    let no_item = [title.clone(), format!("say({})", lua_string(&form.no_item_text)), s("return")];
    let success = [
        s("local progress = pc.getqf(\"collect_count\") + 1"),
        s("pc.setqf(\"collect_count\", progress)"),
        title.clone(),
        format!("say(string.format({}, progress, {required_successes}))", lua_string(&form.progress_text)),
    ];
    let fail = [title, format!("say({})", lua_string(&form.fail_text))];

    [
        format!("when {} begin", chat_trigger(npc, &form.chat_label)),
        format!("\tif pc.getqf(\"collect_count\") >= {required_successes} then"),
        indent(&complete, 2),
        s("\tend"),
        format!("\tif pc.count_item({required_vnum}) < {per_attempt} then"),
        indent(&no_item, 2),
        s("\tend"),
        format!("\tpc.remove_item({required_vnum}, {per_attempt})"),
        s("\tlocal roll = number(1, 100)"),
        format!("\tif roll <= {chance} then"),
        indent(&success, 2),
        s("\telse"),
        indent(&fail, 2),
        s("\tend"),
        s("end"),
    ]
    .join("\n")
}

fn kill_template(form: &QuestFormState) -> String {
    let npc = number_or0(&form.npc_vnum);
    let mob = num_str(number_or0(&form.mob_vnum));
    let kills = num_str(max_or(1.0, &form.required_kills, 1.0));
    let title = format!("say_title({})", lua_string(&form.npc_title));
    let progress = [
        title.clone(),
        format!("say(string.format({}, pc.getqf(\"kill_count\"), {kills}))", lua_string(&form.progress_text)),
    ];
    let mut success = vec![title, format!("say({})", lua_string(&form.success_text))];
    success.extend(reward_lines(form));
    success.push(s("pc.setqf(\"kill_count\", 0)"));
    let trigger = chat_trigger(npc, &form.chat_label);
    [
        format!("when {trigger} with pc.getqf(\"kill_count\") < {kills} begin"),
        indent(&progress, 1),
        s("end"),
        format!("when {trigger} with pc.getqf(\"kill_count\") >= {kills} begin"),
        indent(&success, 1),
        s("end"),
        format!("when {mob}.kill begin"),
        format!("\tif pc.getqf(\"kill_count\") < {kills} then"),
        s("\t\tpc.setqf(\"kill_count\", pc.getqf(\"kill_count\") + 1)"),
        s("\tend"),
        s("end"),
    ]
    .join("\n")
}

fn use_template(form: &QuestFormState) -> String {
    let item = num_str(number_or0(&form.use_item_vnum));
    let mut body = vec![format!("say({})", lua_string(&form.use_text))];
    body.extend(reward_lines(form));
    [format!("when {item}.use begin"), indent(&body, 1), s("end")].join("\n")
}

fn attr_lines(types: &[String; 4], values: &[String; 4]) -> Vec<String> {
    (0..4)
        .filter_map(|i| {
            let apply_type = number_or0(&types[i]);
            let apply_value = number_or0(&values[i]);
            (apply_type > 0.0).then(|| format!("item.set_value({i}, {}, {})", num_str(apply_type), num_str(apply_value)))
        })
        .collect()
}

/// Verschenkt ein bestimmtes Item mit bis zu 4 fest gesetzten
/// Bonus-Attributen (pc.give_item2_select + item.set_value).
fn buffed_item_template(form: &QuestFormState) -> String {
    let npc = number_or0(&form.npc_vnum);
    let item = num_str(number_or0(&form.buffed_item_vnum));
    let count = num_str(max_or(1.0, &form.buffed_item_count, 1.0));
    let mut body = vec![
        format!("say_title({})", lua_string(&form.npc_title)),
        format!("say({})", lua_string(&form.dialog_text)),
        format!("pc.give_item2_select({item}, {count})"),
    ];
    body.extend(attr_lines(&form.buffed_attr_type, &form.buffed_attr_value));
    body.extend(money_line(&form.reward_money));
    [format!("when {} begin", chat_trigger(npc, &form.chat_label)), indent(&body, 1), s("end")].join("\n")
}

/// Für die einfachen Vorlagen (alles außer `Dungeon`/`MultiStep`, die eigene
/// Generatoren haben - dafür liefert diese Funktion wie die TS-Vorschau mit
/// ihrem try/catch einen leeren Text).
pub fn generate_quest_lua(quest_name: &str, t: TemplateType, form: &QuestFormState) -> String {
    let body = match t {
        TemplateType::Dialog => dialog_template(form),
        TemplateType::Collect => collect_template(form),
        TemplateType::ChanceCollect => chance_collect_template(form),
        TemplateType::Kill => kill_template(form),
        TemplateType::Use => use_template(form),
        TemplateType::BuffedItem => buffed_item_template(form),
        TemplateType::Dungeon | TemplateType::MultiStep => return String::new(),
    };
    let body_lines: Vec<&str> = body.split('\n').collect();
    [format!("quest {quest_name} begin"), s("\tstate start begin"), indent(&body_lines, 2), s("\tend"), s("end"), s("")].join("\n")
}

// ---- Dungeon-Run-Vorlage ----
//
// Muster aus drei echten Runs des Servers (Beran_Eintritt.quest,
// Daemonenturm.quest, Teufelskatakomben.quest): NPC-Dialog warpt auf die
// Dungeon-Map, `when enter` startet Etage 1, jeder Etagen-Boss schaltet die
// nächste Etage frei, die letzte vergibt die Belohnung und wirft per
// Server-Timer alle raus.

#[derive(Clone, Debug, PartialEq, Default)]
pub struct DungeonFloor {
    pub regen_file: String,
    pub boss_vnum: String,
    pub entry_x: String,
    pub entry_y: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DungeonFormState {
    pub entry_npc_vnum: String,
    pub entry_chat_label: String,
    pub entry_title: String,
    pub entry_text: String,
    pub required_level: String,
    pub dungeon_map_index: String,
    pub map_index_range_start: String,
    pub map_index_range_end: String,
    pub exit_delay_seconds: String,
    pub reward_item_vnum: String,
    pub reward_item_count: String,
    pub reward_money: String,
    pub floors: Vec<DungeonFloor>,
}

impl Default for DungeonFormState {
    fn default() -> Self {
        Self {
            entry_npc_vnum: s(""),
            entry_chat_label: s("Run"),
            entry_title: s(""),
            entry_text: s(""),
            required_level: s("0"),
            dungeon_map_index: s(""),
            map_index_range_start: s(""),
            map_index_range_end: s(""),
            exit_delay_seconds: s("30"),
            reward_item_vnum: s(""),
            reward_item_count: s("1"),
            reward_money: s(""),
            floors: vec![DungeonFloor::default()],
        }
    }
}

fn in_range_guard(form: &DungeonFormState) -> String {
    format!(
        "pc.get_map_index() >= {} and pc.get_map_index() < {}",
        num_str(number_or0(&form.map_index_range_start)),
        num_str(number_or0(&form.map_index_range_end))
    )
}

pub fn generate_dungeon_quest(quest_name: &str, form: &DungeonFormState) -> String {
    let npc = number_or0(&form.entry_npc_vnum);
    let dungeon_map = num_str(number_or0(&form.dungeon_map_index));
    let required_level = number_or0(&form.required_level);
    let exit_delay = num_str(0f64.max(number_or0(&form.exit_delay_seconds)));
    let default_floors = [DungeonFloor::default()];
    let floors: &[DungeonFloor] = if form.floors.is_empty() { &default_floors } else { &form.floors };
    let n = |v: &str| num_str(number_or0(v));

    let mut lines: Vec<String> = vec![];
    lines.push(format!("quest {quest_name} begin"));
    lines.push(s("\tstate start begin"));

    lines.push(s("\t\t-- Einstieg: NPC-Dialog warpt auf die Dungeon-Map."));
    let mut entry: Vec<String> = vec![];
    if required_level > 0.0 {
        entry.push(format!("if pc.get_level() < {} then", num_str(required_level)));
        entry.push(format!("\tsay_title({})", lua_string(&form.entry_title)));
        entry.push(s("\tsay(\"Du bist noch nicht stark genug für diesen Run.\")"));
        entry.push(s("\treturn"));
        entry.push(s("end"));
    }
    entry.push(format!("say_title({})", lua_string(&form.entry_title)));
    entry.push(format!("say({})", lua_string(&form.entry_text)));
    entry.push(s("local enter = select(\"Betreten\", \"Abbrechen\")"));
    entry.push(s("if enter == 1 then"));
    entry.push(format!("\tpc.warp({} * 100, {} * 100, {dungeon_map})", n(&floors[0].entry_x), n(&floors[0].entry_y)));
    entry.push(s("end"));
    lines.push(format!("\t\twhen {} begin", chat_trigger(npc, &form.entry_chat_label)));
    lines.push(indent(&entry, 3));
    lines.push(s("\t\tend"));
    lines.push(s(""));

    lines.push(s("\t\t-- Ankunft in der Instanz: Etage 1 vorbereiten."));
    lines.push(s("\t\twhen enter begin"));
    lines.push(indent(
        &[
            format!("if {} then", in_range_guard(form)),
            s("\td.setf(\"level\", 1)"),
            format!("\td.regen_file({})", lua_string(&floors[0].regen_file)),
            s("end"),
        ],
        3,
    ));
    lines.push(s("\t\tend"));
    lines.push(s(""));

    for (index, floor) in floors.iter().enumerate() {
        let level = index + 1;
        let is_last = index == floors.len() - 1;
        let boss = n(&floor.boss_vnum);
        lines.push(format!(
            "\t\t-- Etage {level}: {}",
            if is_last { "Boss besiegen -> Run abschließen." } else { "Boss besiegen -> nächste Etage." }
        ));
        lines.push(format!(
            "\t\twhen {boss}.kill with pc.in_dungeon() and {} and d.getf(\"level\") == {level} begin",
            in_range_guard(form)
        ));
        let mut body = vec![s("d.clear_regen()")];
        if !is_last {
            let next = &floors[index + 1];
            body.push(format!("d.setf(\"level\", {})", level + 1));
            body.push(format!("d.jump_all({}, {})", n(&next.entry_x), n(&next.entry_y)));
            body.push(format!("d.regen_file({})", lua_string(&next.regen_file)));
        } else {
            let reward_vnum = number(&form.reward_item_vnum);
            if reward_vnum.is_finite() && reward_vnum > 0.0 {
                let count = max_or(1.0, &form.reward_item_count, 1.0);
                body.push(format!("game.drop_item_with_ownership({}, {})", num_str(reward_vnum), num_str(count)));
            }
            body.extend(money_line(&form.reward_money));
            body.push(s("notice_multiline(\"Der Run wurde erfolgreich abgeschlossen!\", d.notice)"));
            body.push(format!("server_timer(\"{quest_name}_exit\", {exit_delay}, d.get_map_index())"));
        }
        lines.push(indent(&body, 3));
        lines.push(s("\t\tend"));
        lines.push(s(""));
    }

    lines.push(s("\t\t-- Nach der Belohnung alle Spieler aus der Instanz werfen."));
    lines.push(format!("\t\twhen {quest_name}_exit.server_timer begin"));
    lines.push(indent(&["if d.select(get_server_timer_arg()) then", "\td.exit_all()", "end"], 3));
    lines.push(s("\t\tend"));

    lines.push(s("\tend"));
    lines.push(s("end"));
    lines.push(s(""));
    lines.join("\n")
}

// ---- Mehrschritt-Vorlage ----
//
// Jeder Schritt ist ein eigener `state <name> begin...end`-Block, der
// Übergang läuft über `set_state(<name>)` (im Server-Quellcode verifiziert:
// questlua_quest.cpp/questpc.cpp). `__COMPLETE__` ist reine
// Namenskonvention. pc.getqf/setqf sind NICHT pro Zustand gescoped - jeder
// Kill-Schritt bekommt deshalb seinen eigenen Zähler (kill_count_stepN).

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum StepKind {
    Dialog,
    Collect,
    Kill,
    Use,
}

impl StepKind {
    pub const ALL: [StepKind; 4] = [StepKind::Dialog, StepKind::Collect, StepKind::Kill, StepKind::Use];

    pub fn id(self) -> &'static str {
        match self {
            StepKind::Dialog => "dialog",
            StepKind::Collect => "collect",
            StepKind::Kill => "kill",
            StepKind::Use => "use",
        }
    }

    pub fn from_id(id: &str) -> Option<StepKind> {
        Self::ALL.iter().copied().find(|k| k.id() == id)
    }

    pub fn label(self) -> &'static str {
        match self {
            StepKind::Dialog => "NPC-Dialog (mit optionaler Zwischenbelohnung)",
            StepKind::Collect => "Items abgeben",
            StepKind::Kill => "Monster töten",
            StepKind::Use => "Item benutzen",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuestStep {
    pub kind: StepKind,
    pub npc_vnum: String,
    pub chat_label: String,
    pub npc_title: String,
    pub dialog_text: String,
    pub required_item_vnum: String,
    pub required_item_count: String,
    pub success_text: String,
    pub fail_text: String,
    pub mob_vnum: String,
    pub required_kills: String,
    pub progress_text: String,
    pub use_item_vnum: String,
    pub use_text: String,
    /// Optionale Zwischenbelohnung - bei jeder Schritt-Art nutzbar.
    pub reward_item_vnum: String,
    pub reward_item_count: String,
    pub reward_money: String,
    /// Feste Bonus-Attribute auf dem Belohnungs-Item ("0" = ungenutzt).
    pub reward_attr_type: [String; 4],
    pub reward_attr_value: [String; 4],
}

impl Default for QuestStep {
    fn default() -> Self {
        Self {
            kind: StepKind::Dialog,
            npc_vnum: s(""),
            chat_label: s("Quest"),
            npc_title: s(""),
            dialog_text: s(""),
            required_item_vnum: s(""),
            required_item_count: s("1"),
            success_text: s(""),
            fail_text: s("Das reicht noch nicht."),
            mob_vnum: s(""),
            required_kills: s("10"),
            progress_text: s("Du hast %d von %d erledigt."),
            use_item_vnum: s(""),
            use_text: s(""),
            reward_item_vnum: s(""),
            reward_item_count: s("1"),
            reward_money: s(""),
            reward_attr_type: zeros(),
            reward_attr_value: zeros(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MultiStepFormState {
    pub steps: Vec<QuestStep>,
    /// Wiederholbare Quest mit Cooldown (z.B. "alle 4 Tage") statt einmalig.
    pub repeatable: bool,
    pub cooldown_days: String,
}

pub const DEFAULT_COOLDOWN_DAYS: &str = "4";

impl Default for MultiStepFormState {
    fn default() -> Self {
        Self { steps: vec![QuestStep::default()], repeatable: false, cooldown_days: s(DEFAULT_COOLDOWN_DAYS) }
    }
}

/// Erster Schritt heißt immer "start", alle weiteren "step_N" (1-basiert).
fn state_name(index: usize) -> String {
    if index == 0 {
        s("start")
    } else {
        format!("step_{}", index + 1)
    }
}

fn next_state_name(index: usize, is_last: bool, repeatable: bool) -> String {
    if is_last {
        return s(if repeatable { "start" } else { "__COMPLETE__" });
    }
    state_name(index + 1)
}

fn step_reward_lines(step: &QuestStep) -> Vec<String> {
    let mut lines = vec![];
    let item_vnum = number(&step.reward_item_vnum);
    let has_attrs = step.reward_attr_type.iter().any(|t| number_or0(t) > 0.0);
    if item_vnum.is_finite() && item_vnum > 0.0 {
        let count = num_str(max_or(1.0, &step.reward_item_count, 1.0));
        if has_attrs {
            lines.push(format!("pc.give_item2_select({}, {count})", num_str(item_vnum)));
            lines.extend(attr_lines(&step.reward_attr_type, &step.reward_attr_value));
        } else {
            lines.push(format!("pc.give_item2({}, {count})", num_str(item_vnum)));
        }
    }
    lines.extend(money_line(&step.reward_money));
    lines
}

/// Cooldown-Bedingung für den Eintritt in Schritt 0 einer wiederholbaren Quest.
fn cooldown_guard_expr(days: f64) -> String {
    format!(
        "pc.getqf(\"last_complete\") == 0 or get_time() - pc.getqf(\"last_complete\") >= {}",
        num_str(0f64.max(days) * 86400.0)
    )
}

fn cooldown_wait_block(trigger: &str, guard: &str) -> Vec<String> {
    vec![
        format!("when {trigger} with not ({guard}) begin"),
        indent(&[format!("say({})", lua_string("Diese Quest ist noch nicht wieder verfügbar."))], 1),
        s("end"),
    ]
}

fn with_guard(guard: Option<&str>) -> String {
    guard.map(|g| format!(" with {g}")).unwrap_or_default()
}

fn finish(mut lines: Vec<String>, trigger: &str, guard: Option<&str>) -> Vec<String> {
    if let Some(g) = guard {
        lines.extend(cooldown_wait_block(trigger, g));
    }
    lines
}

fn dialog_step_lines(step: &QuestStep, next: &str, extra: &[String], guard: Option<&str>) -> Vec<String> {
    let trigger = chat_trigger(number_or0(&step.npc_vnum), &step.chat_label);
    let mut body = vec![format!("say_title({})", lua_string(&step.npc_title)), format!("say({})", lua_string(&step.dialog_text))];
    body.extend(step_reward_lines(step));
    body.extend(extra.iter().cloned());
    body.push(format!("set_state({next})"));
    let lines = vec![format!("when {trigger}{} begin", with_guard(guard)), indent(&body, 1), s("end")];
    finish(lines, &trigger, guard)
}

fn collect_step_lines(step: &QuestStep, next: &str, extra: &[String], guard: Option<&str>) -> Vec<String> {
    let required_vnum = num_str(number_or0(&step.required_item_vnum));
    let required_count = num_str(max_or(1.0, &step.required_item_count, 1.0));
    let trigger = chat_trigger(number_or0(&step.npc_vnum), &step.chat_label);
    let mut success = vec![
        format!("say_title({})", lua_string(&step.npc_title)),
        format!("say({})", lua_string(&step.success_text)),
        format!("pc.remove_item({required_vnum}, {required_count})"),
    ];
    success.extend(step_reward_lines(step));
    success.extend(extra.iter().cloned());
    success.push(format!("set_state({next})"));
    let fail = [format!("say_title({})", lua_string(&step.npc_title)), format!("say({})", lua_string(&step.fail_text))];
    let lines = vec![
        format!("when {trigger}{} begin", with_guard(guard)),
        format!("\tif pc.count_item({required_vnum}) >= {required_count} then"),
        indent(&success, 2),
        s("\telse"),
        indent(&fail, 2),
        s("\tend"),
        s("end"),
    ];
    finish(lines, &trigger, guard)
}

fn kill_step_lines(step: &QuestStep, index: usize, next: &str, extra: &[String], guard: Option<&str>) -> Vec<String> {
    let mob = num_str(number_or0(&step.mob_vnum));
    let kills = num_str(max_or(1.0, &step.required_kills, 1.0));
    let qf = lua_string(&format!("kill_count_step{index}"));
    let trigger = chat_trigger(number_or0(&step.npc_vnum), &step.chat_label);
    let progress = [
        format!("say_title({})", lua_string(&step.npc_title)),
        format!("say(string.format({}, pc.getqf({qf}), {kills}))", lua_string(&step.progress_text)),
    ];
    let mut success = vec![format!("say_title({})", lua_string(&step.npc_title)), format!("say({})", lua_string(&step.success_text))];
    success.extend(step_reward_lines(step));
    success.extend(extra.iter().cloned());
    success.push(format!("set_state({next})"));
    let guard_suffix = guard.map(|g| format!(" and ({g})")).unwrap_or_default();
    let lines = vec![
        format!("when {trigger} with pc.getqf({qf}) < {kills}{guard_suffix} begin"),
        indent(&progress, 1),
        s("end"),
        format!("when {trigger} with pc.getqf({qf}) >= {kills}{guard_suffix} begin"),
        indent(&success, 1),
        s("end"),
        format!("when {mob}.kill begin"),
        format!("\tif pc.getqf({qf}) < {kills} then"),
        format!("\t\tpc.setqf({qf}, pc.getqf({qf}) + 1)"),
        s("\tend"),
        s("end"),
    ];
    finish(lines, &trigger, guard)
}

fn use_step_lines(step: &QuestStep, next: &str, extra: &[String], guard: Option<&str>) -> Vec<String> {
    let trigger = format!("{}.use", num_str(number_or0(&step.use_item_vnum)));
    let mut body = vec![format!("say({})", lua_string(&step.use_text))];
    body.extend(step_reward_lines(step));
    body.extend(extra.iter().cloned());
    body.push(format!("set_state({next})"));
    let lines = vec![format!("when {trigger}{} begin", with_guard(guard)), indent(&body, 1), s("end")];
    finish(lines, &trigger, guard)
}

pub fn generate_multi_step_quest(quest_name: &str, form: &MultiStepFormState) -> String {
    let default_steps = [QuestStep::default()];
    let steps: &[QuestStep] = if form.steps.is_empty() { &default_steps } else { &form.steps };
    let cooldown_days = 0f64.max(number_or0(&form.cooldown_days));
    let cooldown_guard = form.repeatable.then(|| cooldown_guard_expr(cooldown_days));

    let mut lines = vec![format!("quest {quest_name} begin")];
    for (index, step) in steps.iter().enumerate() {
        let is_last = index == steps.len() - 1;
        let next = next_state_name(index, is_last, form.repeatable);
        let mut extra: Vec<String> = vec![];
        if is_last && form.repeatable {
            extra.push(s("pc.setqf(\"last_complete\", get_time())"));
            for (i, st) in steps.iter().enumerate() {
                if st.kind == StepKind::Kill {
                    extra.push(format!("pc.setqf({}, 0)", lua_string(&format!("kill_count_step{i}"))));
                }
            }
        }
        lines.push(format!("\t-- Schritt {}{}", index + 1, if is_last { " (Abschluss)" } else { "" }));
        lines.push(format!("\tstate {} begin", state_name(index)));
        let guard = if index == 0 { cooldown_guard.as_deref() } else { None };
        let step_lines = match step.kind {
            StepKind::Dialog => dialog_step_lines(step, &next, &extra, guard),
            StepKind::Collect => collect_step_lines(step, &next, &extra, guard),
            StepKind::Kill => kill_step_lines(step, index, &next, &extra, guard),
            StepKind::Use => use_step_lines(step, &next, &extra, guard),
        };
        lines.push(indent(&step_lines, 2));
        lines.push(s("\tend"));
        lines.push(s(""));
    }
    // Terminaler Zustand - bei einer wiederholbaren Quest nie erreicht.
    if !form.repeatable {
        lines.push(s("\tstate __COMPLETE__ begin"));
        lines.push(s("\tend"));
    }
    lines.push(s("end"));
    lines.push(s(""));
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Zählt `keyword` als ganzes Wort (wie `\bkeyword\b` in JS).
    fn count_keyword(text: &str, keyword: &str) -> usize {
        let is_word = |c: char| c.is_ascii_alphanumeric() || c == '_';
        let mut n = 0;
        for (i, _) in text.match_indices(keyword) {
            let before = text[..i].chars().next_back();
            let after = text[i + keyword.len()..].chars().next();
            if !before.is_some_and(is_word) && !after.is_some_and(is_word) {
                n += 1;
            }
        }
        n
    }

    /// Jedes `begin` und jedes `if` braucht genau ein passendes `end`.
    fn expect_balanced(lua: &str) {
        let opens = count_keyword(lua, "begin") + count_keyword(lua, "if");
        let ends = count_keyword(lua, "end");
        assert_eq!(ends, opens, "unbalanciert:\n{lua}");
    }

    fn with_npc(patch: impl FnOnce(&mut QuestFormState)) -> QuestFormState {
        let mut f = QuestFormState { npc_vnum: s("20349"), npc_title: s("Testwirt"), ..Default::default() };
        patch(&mut f);
        f
    }

    #[test]
    fn wraps_every_template_in_one_quest_state_block() {
        let lua = generate_quest_lua("test_quest", TemplateType::Dialog, &with_npc(|f| f.dialog_text = s("Hallo")));
        assert!(lua.starts_with("quest test_quest begin"));
        assert!(lua.contains("state start begin"));
        expect_balanced(&lua);
    }

    #[test]
    fn dialog_says_text_and_gives_reward() {
        let lua = generate_quest_lua(
            "dialog_quest",
            TemplateType::Dialog,
            &with_npc(|f| {
                f.dialog_text = s("Willkommen!");
                f.reward_item_vnum = s("40");
                f.reward_item_count = s("3");
            }),
        );
        assert!(lua.contains("when 20349.chat.\"Quest\" begin"));
        assert!(lua.contains("say(\"Willkommen!\")"));
        assert!(lua.contains("pc.give_item2(40, 3)"));
        expect_balanced(&lua);
    }

    #[test]
    fn collect_gates_reward_and_removes_item() {
        let lua = generate_quest_lua(
            "collect_quest",
            TemplateType::Collect,
            &with_npc(|f| {
                f.required_item_vnum = s("100");
                f.required_item_count = s("5");
            }),
        );
        assert!(lua.contains("if pc.count_item(100) >= 5 then"));
        assert!(lua.contains("pc.remove_item(100, 5)"));
        expect_balanced(&lua);
    }

    #[test]
    fn chance_collect_consumes_before_roll() {
        let lua = generate_quest_lua(
            "bio_quest",
            TemplateType::ChanceCollect,
            &with_npc(|f| {
                f.required_item_vnum = s("200");
                f.items_per_attempt = s("2");
                f.accept_chance_percent = s("60");
                f.required_successes = s("9");
            }),
        );
        let remove_idx = lua.find("pc.remove_item(200, 2)").expect("remove");
        let roll_idx = lua.find("local roll = number(1, 100)").expect("roll");
        assert!(roll_idx > remove_idx);
        assert!(lua.contains("if roll <= 60 then"));
        assert!(lua.contains("if pc.getqf(\"collect_count\") >= 9 then"));
        expect_balanced(&lua);
    }

    #[test]
    fn kill_tracks_progress() {
        let lua = generate_quest_lua(
            "kill_quest",
            TemplateType::Kill,
            &with_npc(|f| {
                f.mob_vnum = s("101");
                f.required_kills = s("5");
            }),
        );
        assert!(lua.contains("when 101.kill begin"));
        assert!(lua.contains("pc.getqf(\"kill_count\")"));
        expect_balanced(&lua);
    }

    #[test]
    fn use_triggers_on_item_use() {
        let lua = generate_quest_lua(
            "use_quest",
            TemplateType::Use,
            &with_npc(|f| {
                f.use_item_vnum = s("300");
                f.use_text = s("Bumm!");
            }),
        );
        assert!(lua.contains("when 300.use begin"));
        assert!(lua.contains("say(\"Bumm!\")"));
        expect_balanced(&lua);
    }

    #[test]
    fn buffed_item_sets_only_filled_slots() {
        let lua = generate_quest_lua(
            "buffed_quest",
            TemplateType::BuffedItem,
            &with_npc(|f| {
                f.buffed_item_vnum = s("800009");
                f.buffed_item_count = s("1");
                f.buffed_attr_type[0] = s("4");
                f.buffed_attr_value[0] = s("500");
                f.buffed_attr_type[1] = s("5");
                f.buffed_attr_value[1] = s("700");
            }),
        );
        assert!(lua.contains("pc.give_item2_select(800009, 1)"));
        assert!(lua.contains("item.set_value(0, 4, 500)"));
        assert!(lua.contains("item.set_value(1, 5, 700)"));
        assert!(!lua.contains("item.set_value(2,"));
        assert!(!lua.contains("item.set_value(3,"));
        expect_balanced(&lua);
    }

    #[test]
    fn escapes_quotes_and_newlines() {
        let lua = generate_quest_lua("escape_quest", TemplateType::Dialog, &with_npc(|f| f.dialog_text = s("Er sagt \"Hallo\"\nund geht.")));
        assert!(lua.contains("say(\"Er sagt \\\"Hallo\\\"[ENTER]und geht.\")"));
    }

    fn two_floor_form() -> DungeonFormState {
        DungeonFormState {
            entry_npc_vnum: s("9001"),
            dungeon_map_index: s("150"),
            map_index_range_start: s("660000"),
            map_index_range_end: s("670000"),
            floors: vec![
                DungeonFloor { boss_vnum: s("8031"), regen_file: s("data/dungeon/x/floor1.txt"), entry_x: s("100"), entry_y: s("100") },
                DungeonFloor { boss_vnum: s("8032"), regen_file: s("data/dungeon/x/floor2.txt"), entry_x: s("200"), entry_y: s("200") },
            ],
            reward_item_vnum: s("50"),
            reward_item_count: s("1"),
            ..Default::default()
        }
    }

    #[test]
    fn dungeon_balanced() {
        expect_balanced(&generate_dungeon_quest("test_run", &two_floor_form()));
    }

    #[test]
    fn dungeon_only_last_floor_exits() {
        let lua = generate_dungeon_quest("test_run", &two_floor_form());
        assert!(lua.contains("when 8031.kill with pc.in_dungeon()"));
        assert!(lua.contains("when 8032.kill with pc.in_dungeon()"));
        assert!(lua.contains("d.regen_file(\"data/dungeon/x/floor2.txt\")"));
        assert!(lua.contains("d.jump_all(200, 200)"));
        assert!(lua.contains("when test_run_exit.server_timer begin"));
        assert_eq!(lua.matches("d.exit_all()").count(), 1);
    }

    #[test]
    fn dungeon_respects_map_range() {
        let lua = generate_dungeon_quest("test_run", &two_floor_form());
        assert!(lua.matches("pc.get_map_index() >= 660000 and pc.get_map_index() < 670000").count() >= 2);
    }

    #[test]
    fn dungeon_falls_back_to_default_floor() {
        let lua = generate_dungeon_quest("empty_run", &DungeonFormState { floors: vec![], ..two_floor_form() });
        expect_balanced(&lua);
        assert!(lua.contains("when 0.kill with pc.in_dungeon()"));
    }

    fn base_step(kind: StepKind) -> QuestStep {
        QuestStep { kind, npc_vnum: s("9001"), chat_label: s("Quest"), npc_title: s("Testwirt"), ..Default::default() }
    }
    fn dialog_step() -> QuestStep {
        QuestStep { dialog_text: s("Hallo!"), ..base_step(StepKind::Dialog) }
    }
    fn collect_step() -> QuestStep {
        QuestStep { required_item_vnum: s("100"), required_item_count: s("5"), ..base_step(StepKind::Collect) }
    }
    fn kill_step() -> QuestStep {
        QuestStep { mob_vnum: s("101"), required_kills: s("5"), ..base_step(StepKind::Kill) }
    }
    fn use_step() -> QuestStep {
        QuestStep { kind: StepKind::Use, use_item_vnum: s("300"), use_text: s("Bumm!"), ..Default::default() }
    }
    fn form(steps: Vec<QuestStep>) -> MultiStepFormState {
        MultiStepFormState { steps, repeatable: false, cooldown_days: s("4") }
    }
    fn repeat_form(steps: Vec<QuestStep>, days: &str) -> MultiStepFormState {
        MultiStepFormState { steps, repeatable: true, cooldown_days: s(days) }
    }

    #[test]
    fn multi_wraps_each_step_and_complete_state() {
        let lua = generate_multi_step_quest("multi_quest", &form(vec![dialog_step(), collect_step(), kill_step()]));
        assert!(lua.starts_with("quest multi_quest begin"));
        assert!(lua.contains("state start begin"));
        assert!(lua.contains("state step_2 begin"));
        assert!(lua.contains("state step_3 begin"));
        assert!(lua.contains("state __COMPLETE__ begin"));
        expect_balanced(&lua);
    }

    #[test]
    fn multi_dialog_advances() {
        let lua = generate_multi_step_quest("d", &form(vec![dialog_step(), use_step()]));
        assert!(lua.contains("when 9001.chat.\"Quest\" begin"));
        assert!(lua.contains("set_state(step_2)"));
        expect_balanced(&lua);
    }

    #[test]
    fn multi_collect_step() {
        let lua = generate_multi_step_quest("c", &form(vec![collect_step(), use_step()]));
        assert!(lua.contains("when 9001.chat.\"Quest\" begin"));
        assert!(lua.contains("if pc.count_item(100) >= 5 then"));
        assert!(lua.contains("pc.remove_item(100, 5)"));
        assert!(lua.contains("set_state(step_2)"));
        expect_balanced(&lua);
    }

    #[test]
    fn multi_kill_uses_namespaced_counter() {
        let lua = generate_multi_step_quest("k", &form(vec![kill_step(), use_step()]));
        assert!(lua.contains("when 101.kill begin"));
        assert!(lua.contains("pc.getqf(\"kill_count_step0\")"));
        assert!(!lua.contains("pc.getqf(\"kill_count\")"));
        expect_balanced(&lua);
    }

    #[test]
    fn multi_use_last_completes() {
        let lua = generate_multi_step_quest("u", &form(vec![dialog_step(), use_step()]));
        assert!(lua.contains("when 300.use begin"));
        assert!(lua.contains("say(\"Bumm!\")"));
        assert!(lua.contains("set_state(__COMPLETE__)"));
        expect_balanced(&lua);
    }

    #[test]
    fn multi_chains_three_steps() {
        let lua = generate_multi_step_quest("chain", &form(vec![dialog_step(), collect_step(), kill_step()]));
        assert!(lua.contains("set_state(step_2)"));
        assert!(lua.contains("set_state(step_3)"));
        assert!(lua.contains("set_state(__COMPLETE__)"));
        assert!(!lua.contains("state step_4 begin"));
        expect_balanced(&lua);
    }

    #[test]
    fn multi_two_kill_steps_distinct_counters() {
        let lua = generate_multi_step_quest(
            "double_kill",
            &form(vec![kill_step(), dialog_step(), QuestStep { mob_vnum: s("202"), ..kill_step() }]),
        );
        assert!(lua.contains("pc.getqf(\"kill_count_step0\")"));
        assert!(lua.contains("pc.getqf(\"kill_count_step2\")"));
        expect_balanced(&lua);
    }

    #[test]
    fn multi_empty_falls_back() {
        let lua = generate_multi_step_quest("empty_multi", &form(vec![]));
        expect_balanced(&lua);
        assert!(lua.contains("state start begin"));
        assert!(lua.contains("state __COMPLETE__ begin"));
    }

    #[test]
    fn repeatable_gates_only_step0() {
        let lua = generate_multi_step_quest("repeat_quest", &repeat_form(vec![dialog_step(), use_step()], "4"));
        let expr = "pc.getqf(\"last_complete\") == 0 or get_time() - pc.getqf(\"last_complete\") >= 345600";
        assert!(lua.contains(&format!("when 9001.chat.\"Quest\" with {expr} begin")));
        assert!(lua.contains(&format!("when 9001.chat.\"Quest\" with not ({expr}) begin")));
        assert!(lua.contains("when 300.use begin"));
        assert!(!lua.contains("300.use with"));
        expect_balanced(&lua);
    }

    #[test]
    fn repeatable_loops_back() {
        let lua = generate_multi_step_quest("repeat_quest", &repeat_form(vec![dialog_step(), use_step()], "1"));
        assert!(lua.contains("set_state(start)"));
        assert!(!lua.contains("__COMPLETE__"));
        assert!(lua.contains("pc.setqf(\"last_complete\", get_time())"));
        expect_balanced(&lua);
    }

    #[test]
    fn repeatable_resets_kill_counters() {
        let lua = generate_multi_step_quest(
            "repeat_kill",
            &repeat_form(vec![kill_step(), dialog_step(), QuestStep { mob_vnum: s("202"), ..kill_step() }], "2"),
        );
        assert!(lua.contains("pc.setqf(\"kill_count_step0\", 0)"));
        assert!(lua.contains("pc.setqf(\"kill_count_step2\", 0)"));
        expect_balanced(&lua);
    }

    #[test]
    fn repeatable_kill_first_gates_all_chat_triggers() {
        let lua = generate_multi_step_quest("repeat_kill_first", &repeat_form(vec![kill_step(), use_step()], "1"));
        let expr = "pc.getqf(\"last_complete\") == 0 or get_time() - pc.getqf(\"last_complete\") >= 86400";
        assert!(lua.contains(&format!("pc.getqf(\"kill_count_step0\") < 5 and ({expr})")));
        assert!(lua.contains(&format!("pc.getqf(\"kill_count_step0\") >= 5 and ({expr})")));
        assert!(lua.contains(&format!("when 9001.chat.\"Quest\" with not ({expr}) begin")));
        expect_balanced(&lua);
    }

    #[test]
    fn attr_reward_uses_give_item2_select() {
        let mut st = collect_step();
        st.reward_item_vnum = s("700010");
        st.reward_item_count = s("1");
        st.reward_attr_type[0] = s("1");
        st.reward_attr_value[0] = s("250");
        let lua = generate_multi_step_quest("attr_reward", &form(vec![st]));
        assert!(lua.contains("pc.give_item2_select(700010, 1)"));
        assert!(lua.contains("item.set_value(0, 1, 250)"));
        assert!(!lua.contains("pc.give_item2(700010"));
        expect_balanced(&lua);
    }

    #[test]
    fn plain_reward_without_attrs() {
        let st = QuestStep { reward_item_vnum: s("40"), reward_item_count: s("3"), ..collect_step() };
        let lua = generate_multi_step_quest("plain_reward", &form(vec![st]));
        assert!(lua.contains("pc.give_item2(40, 3)"));
        assert!(!lua.contains("item.set_value"));
        expect_balanced(&lua);
    }
}
