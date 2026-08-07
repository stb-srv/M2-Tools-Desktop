// Lua generators for the Quest Builder's "template" mode. Every construct
// used here (when NPC.chat."label" [with cond] begin...end, when MOB.kill
// begin...end, when ITEM.use begin...end, say/say_title, pc.give_item2,
// pc.changemoney, pc.count_item/remove_item, pc.getqf/setqf, string.format)
// was verified against real quest source files on the user's dev server
// (Pferd/Pferd_rufen.lua, Biologie/Orkzahn.lua) - not guessed from generic
// Metin2 documentation. Templates only cover the common single-state cases;
// anything more complex is edited as raw code after creation.

export type TemplateType =
  | "dialog"
  | "collect"
  | "chance_collect"
  | "kill"
  | "use"
  | "buffed_item"
  | "dungeon"
  | "multi_step";

export interface QuestFormState {
  npcVnum: string;
  chatLabel: string;
  npcTitle: string;
  dialogText: string;
  rewardItemVnum: string;
  rewardItemCount: string;
  rewardMoney: string;
  requiredItemVnum: string;
  requiredItemCount: string;
  successText: string;
  failText: string;
  mobVnum: string;
  requiredKills: string;
  progressText: string;
  useItemVnum: string;
  useText: string;
  // chance_collect only ("Biologie"-style: submit item, % chance it counts
  // toward the goal - see chanceCollectTemplate below)
  itemsPerAttempt: string;
  acceptChancePercent: string;
  requiredSuccesses: string;
  noItemText: string;
  completeText: string;
  // buffed_item only - gives a specific item with forced bonus attributes
  // instead of a random/rolled one. `apply_type` values reuse the same
  // EApplyTypes enum as item_proto.applytype0-3 (see itemFlags.ts's
  // APPLY_TYPES) - item.set_value(index, apply_type, value) (bound to
  // CItem::SetForceAttribute in questlua_item.cpp) writes directly into one
  // of the item's 7 bonus-attribute slots, the same slots normally filled
  // by the random rare-item roll. "0"/apply type "Keine" means the slot is
  // left empty.
  buffedItemVnum: string;
  buffedItemCount: string;
  buffedAttrType0: string;
  buffedAttrValue0: string;
  buffedAttrType1: string;
  buffedAttrValue1: string;
  buffedAttrType2: string;
  buffedAttrValue2: string;
  buffedAttrType3: string;
  buffedAttrValue3: string;
}

export const DEFAULT_FORM: QuestFormState = {
  npcVnum: "",
  chatLabel: "Quest",
  npcTitle: "",
  dialogText: "",
  rewardItemVnum: "",
  rewardItemCount: "1",
  rewardMoney: "",
  requiredItemVnum: "",
  requiredItemCount: "1",
  successText: "",
  failText: "Das reicht noch nicht.",
  mobVnum: "",
  requiredKills: "10",
  progressText: "Du hast %d von %d erledigt.",
  useItemVnum: "",
  useText: "",
  itemsPerAttempt: "1",
  acceptChancePercent: "60",
  requiredSuccesses: "10",
  noItemText: "Du hast das benötigte Item nicht dabei.",
  completeText: "Damit hast du genug gesammelt!",
  buffedItemVnum: "",
  buffedItemCount: "1",
  buffedAttrType0: "0",
  buffedAttrValue0: "0",
  buffedAttrType1: "0",
  buffedAttrValue1: "0",
  buffedAttrType2: "0",
  buffedAttrValue2: "0",
  buffedAttrType3: "0",
  buffedAttrValue3: "0",
};

// say()/say_title() take a plain Lua double-quoted string - escape what
// would otherwise break out of it, and use the game's own "[ENTER]" marker
// (seen throughout the real quest files) for user-entered line breaks.
function luaString(text: string): string {
  const escaped = text
    .replace(/\\/g, "\\\\")
    .replace(/"/g, '\\"')
    .replace(/\r\n|\n/g, "[ENTER]");
  return `"${escaped}"`;
}

function indent(lines: string[], depth: number): string {
  const prefix = "\t".repeat(depth);
  return lines.map((l) => (l ? prefix + l : "")).join("\n");
}

function rewardLines(form: QuestFormState): string[] {
  const lines: string[] = [];
  const itemVnum = Number(form.rewardItemVnum);
  if (Number.isFinite(itemVnum) && itemVnum > 0) {
    const count = Math.max(1, Number(form.rewardItemCount) || 1);
    lines.push(`pc.give_item2(${itemVnum}, ${count})`);
  }
  const money = Number(form.rewardMoney);
  if (Number.isFinite(money) && money !== 0) {
    lines.push(`pc.changemoney(${money})`);
  }
  return lines;
}

function dialogTemplate(form: QuestFormState): string {
  const npcVnum = Number(form.npcVnum) || 0;
  const body = [
    `say_title(${luaString(form.npcTitle)})`,
    `say(${luaString(form.dialogText)})`,
    ...rewardLines(form),
  ];
  return [
    `when ${npcVnum}.chat.${luaString(form.chatLabel)} begin`,
    indent(body, 1),
    `end`,
  ].join("\n");
}

function collectTemplate(form: QuestFormState): string {
  const npcVnum = Number(form.npcVnum) || 0;
  const requiredVnum = Number(form.requiredItemVnum) || 0;
  const requiredCount = Math.max(1, Number(form.requiredItemCount) || 1);
  const successBody = [
    `say_title(${luaString(form.npcTitle)})`,
    `say(${luaString(form.successText)})`,
    `pc.remove_item(${requiredVnum}, ${requiredCount})`,
    ...rewardLines(form),
  ];
  const failBody = [
    `say_title(${luaString(form.npcTitle)})`,
    `say(${luaString(form.failText)})`,
  ];
  return [
    `when ${npcVnum}.chat.${luaString(form.chatLabel)} begin`,
    `\tif pc.count_item(${requiredVnum}) >= ${requiredCount} then`,
    indent(successBody, 2),
    `\telse`,
    indent(failBody, 2),
    `\tend`,
    `end`,
  ].join("\n");
}

// "Biologie"-Sammel-Quest: submitting the item doesn't guarantee progress -
// each attempt consumes the item and rolls a % chance to actually count.
// Verified directly against Biologie/Orkzahn.lua on the server: it consumes
// the item first (`pc.remove_item`), then `local s = number(1,100)` against
// a `pass_percent`, and on success bumps a `pc.getqf`/`pc.setqf` counter
// until a target count is reached - real quests vary the percent based on
// buffs/pets, this template keeps it as one fixed value the user sets.
function chanceCollectTemplate(form: QuestFormState): string {
  const npcVnum = Number(form.npcVnum) || 0;
  const requiredVnum = Number(form.requiredItemVnum) || 0;
  const itemsPerAttempt = Math.max(1, Number(form.itemsPerAttempt) || 1);
  const chance = Math.min(100, Math.max(0, Number(form.acceptChancePercent) || 0));
  const requiredSuccesses = Math.max(1, Number(form.requiredSuccesses) || 1);

  const completeBody = [
    `say_title(${luaString(form.npcTitle)})`,
    `say(${luaString(form.completeText)})`,
    ...rewardLines(form),
    `pc.setqf("collect_count", 0)`,
    `return`,
  ];
  const noItemBody = [
    `say_title(${luaString(form.npcTitle)})`,
    `say(${luaString(form.noItemText)})`,
    `return`,
  ];
  const successBody = [
    `local progress = pc.getqf("collect_count") + 1`,
    `pc.setqf("collect_count", progress)`,
    `say_title(${luaString(form.npcTitle)})`,
    `say(string.format(${luaString(form.progressText)}, progress, ${requiredSuccesses}))`,
  ];
  const failBody = [
    `say_title(${luaString(form.npcTitle)})`,
    `say(${luaString(form.failText)})`,
  ];

  return [
    `when ${npcVnum}.chat.${luaString(form.chatLabel)} begin`,
    `\tif pc.getqf("collect_count") >= ${requiredSuccesses} then`,
    indent(completeBody, 2),
    `\tend`,
    `\tif pc.count_item(${requiredVnum}) < ${itemsPerAttempt} then`,
    indent(noItemBody, 2),
    `\tend`,
    `\tpc.remove_item(${requiredVnum}, ${itemsPerAttempt})`,
    `\tlocal roll = number(1, 100)`,
    `\tif roll <= ${chance} then`,
    indent(successBody, 2),
    `\telse`,
    indent(failBody, 2),
    `\tend`,
    `end`,
  ].join("\n");
}

function killTemplate(form: QuestFormState): string {
  const npcVnum = Number(form.npcVnum) || 0;
  const mobVnum = Number(form.mobVnum) || 0;
  const requiredKills = Math.max(1, Number(form.requiredKills) || 1);
  const progressBody = [
    `say_title(${luaString(form.npcTitle)})`,
    `say(string.format(${luaString(form.progressText)}, pc.getqf("kill_count"), ${requiredKills}))`,
  ];
  const successBody = [
    `say_title(${luaString(form.npcTitle)})`,
    `say(${luaString(form.successText)})`,
    ...rewardLines(form),
    `pc.setqf("kill_count", 0)`,
  ];
  return [
    `when ${npcVnum}.chat.${luaString(form.chatLabel)} with pc.getqf("kill_count") < ${requiredKills} begin`,
    indent(progressBody, 1),
    `end`,
    `when ${npcVnum}.chat.${luaString(form.chatLabel)} with pc.getqf("kill_count") >= ${requiredKills} begin`,
    indent(successBody, 1),
    `end`,
    `when ${mobVnum}.kill begin`,
    `\tif pc.getqf("kill_count") < ${requiredKills} then`,
    `\t\tpc.setqf("kill_count", pc.getqf("kill_count") + 1)`,
    `\tend`,
    `end`,
  ].join("\n");
}

function useTemplate(form: QuestFormState): string {
  const useItemVnum = Number(form.useItemVnum) || 0;
  const body = [`say(${luaString(form.useText)})`, ...rewardLines(form)];
  return [
    `when ${useItemVnum}.use begin`,
    indent(body, 1),
    `end`,
  ].join("\n");
}

// Gives one specific item (e.g. the "+9" step of an already-built refine
// chain) with up to 4 forced bonus-attribute slots, instead of the box/
// loot-table system's vnum+count+probability-only entries, which cannot
// carry attribute values at all. `pc.give_item2_select` (unlike the plain
// `pc.give_item2` every other template uses) additionally marks the new
// item as the quest's "current item", which `item.set_value` then writes
// into - verified against questlua_pc.cpp/questlua_item.cpp, not guessed.
function buffedItemTemplate(form: QuestFormState): string {
  const npcVnum = Number(form.npcVnum) || 0;
  const itemVnum = Number(form.buffedItemVnum) || 0;
  const itemCount = Math.max(1, Number(form.buffedItemCount) || 1);

  const attrSlots: [string, string][] = [
    [form.buffedAttrType0, form.buffedAttrValue0],
    [form.buffedAttrType1, form.buffedAttrValue1],
    [form.buffedAttrType2, form.buffedAttrValue2],
    [form.buffedAttrType3, form.buffedAttrValue3],
  ];

  const body = [
    `say_title(${luaString(form.npcTitle)})`,
    `say(${luaString(form.dialogText)})`,
    `pc.give_item2_select(${itemVnum}, ${itemCount})`,
  ];
  attrSlots.forEach(([type, value], index) => {
    const applyType = Number(type) || 0;
    const applyValue = Number(value) || 0;
    if (applyType > 0) {
      body.push(`item.set_value(${index}, ${applyType}, ${applyValue})`);
    }
  });
  const money = Number(form.rewardMoney);
  if (Number.isFinite(money) && money !== 0) {
    body.push(`pc.changemoney(${money})`);
  }

  return [
    `when ${npcVnum}.chat.${luaString(form.chatLabel)} begin`,
    indent(body, 1),
    `end`,
  ].join("\n");
}

type SimpleTemplateType = Exclude<TemplateType, "dungeon" | "multi_step">;

const TEMPLATE_BODIES: Record<SimpleTemplateType, (form: QuestFormState) => string> = {
  dialog: dialogTemplate,
  collect: collectTemplate,
  chance_collect: chanceCollectTemplate,
  kill: killTemplate,
  use: useTemplate,
  buffed_item: buffedItemTemplate,
};

export function generateQuestLua(
  questName: string,
  type: SimpleTemplateType,
  form: QuestFormState,
): string {
  const body = TEMPLATE_BODIES[type](form);
  return [
    `quest ${questName} begin`,
    `\tstate start begin`,
    indent(body.split("\n"), 2),
    `\tend`,
    `end`,
    "",
  ].join("\n");
}

export const TEMPLATE_LABELS: Record<TemplateType, string> = {
  dialog: "NPC-Dialog mit Belohnung",
  collect: "Sammel-Quest (Items abgeben)",
  chance_collect: "Sammel-Quest mit Erfolgschance (Biologie-Stil)",
  kill: "Kill-Quest (Monster töten)",
  use: "Item-Benutzung",
  buffed_item: "Item mit festen Boni verschenken",
  dungeon: "Run / Instanz-Dungeon (mehrere Etagen)",
  multi_step: "Mehrschritt-Quest (mehrere Aufgaben nacheinander)",
};

export const TEMPLATE_HINTS: Record<TemplateType, string> = {
  dialog: "Spieler spricht den NPC an und bekommt direkt eine Belohnung.",
  collect:
    "Spieler muss eine bestimmte Anzahl eines Items besitzen und beim NPC abgeben, um die Belohnung zu erhalten.",
  chance_collect:
    "Wie bei den echten Biologie-Quests (z.B. Orkzahn.lua): Spieler gibt bei jedem Versuch ein Item ab, das Item wird IMMER verbraucht, zählt aber nur mit einer festgelegten Chance zum Fortschritt. Erst nach genug Erfolgen gibt es die Belohnung.",
  kill: "Spieler muss eine bestimmte Anzahl eines Monsters töten (Fortschritt wird pro Spieler gespeichert), dann beim NPC melden.",
  use: "Ein Item löst beim Benutzen direkt einen Effekt/eine Belohnung aus (kein NPC nötig).",
  buffed_item:
    "Für vorgegebene Boni (z.B. \"Schwert+9 mit INT 500, STR 700\"), die über die Kisten-Loot-Tabelle nicht möglich sind (die kennt nur VNUM+Anzahl+Chance, keine Attribut-Werte). Spieler spricht den NPC an und bekommt ein konkretes Item (z.B. die bereits im Aufwertungs-Editor angelegte \"+9\"-Stufe) mit bis zu 4 fest eingestellten Bonus-Attributen - keine Server-Änderung nötig, item.set_value ist bereits im Quest-Lua vorhanden.",
  dungeon:
    "Grundgerüst für einen mehrstufigen Run wie Beran/Daemonenturm/Teufelskatakomben: NPC-Einstieg, pro Etage ein Boss-Kill schaltet die nächste Etage frei (d.regen_file/d.jump_all/d.setf), am Ende Belohnung + Rauswurf. Deckt bewusst nur das gemeinsame Grundmuster ab - Passwort-Systeme, Zeitlimits o.ä. wie beim echten Beran müssen danach im Code-Editor von Hand ergänzt werden.",
  multi_step:
    "Verkettet beliebig viele der bekannten Bausteine (Dialog, Sammeln, Töten, Item benutzen) zu einer echten Mehrschritt-Quest, ganz ohne Code - z.B. \"Rede mit NPC\" → \"sammle 10 Wolfsfelle\" → \"töte den Boss\" → \"rede nochmal für die Belohnung\". Jeder Schritt wird ein eigener state-Block, der Übergang läuft über set_state() - gegen den echten Server-Quellcode verifiziert (questlua_quest.cpp/questpc.cpp), nicht nur aus dem Wiki übernommen.",
};

// ---- Dungeon-Run-Vorlage ----
//
// Das Muster ist aus drei echten Runs auf dem Server extrahiert (nicht
// angenommen): Beran_Eintritt.quest, Daemonenturm.quest,
// Teufelskatakomben.quest teilen alle dieselbe Grundstruktur -
//   1. NPC-Dialog warpt den Spieler mit `pc.warp(x*100, y*100, mapIndex)`
//      auf die Dungeon-Map.
//   2. `when enter begin` erkennt die Ankunft und startet Etage 1:
//      `d.setf("level", 1)` + `d.regen_file(pfad)`.
//   3. Jede Etage hat einen Boss/Trigger-Vnum. Dessen `.kill`-Event prüft
//      `pc.in_dungeon() and pc.get_map_index() >= start and < ende and
//      d.getf("level") == N`, räumt mit `d.clear_regen()` auf, springt mit
//      `d.jump_all(x, y)` zur nächsten Etage und lädt deren Regen-Datei.
//   4. Die letzte Etage vergibt die Belohnung und wirft nach kurzer
//      Verzögerung mit `d.exit_all()` alle raus.
//
// Was diese Vorlage NICHT abdeckt (im echten Beran per Hand gebaut):
// Passwort-Systeme, Gruppen-Leader-Logik, komplexe Fail-Timer-Ketten,
// Cooldowns über `game.set_event_flag`. Die generierte Datei ist ein
// funktionierender Ausgangspunkt, kein 1:1-Ersatz für diese Spezialfälle.

export interface DungeonFloor {
  regenFile: string;
  bossVnum: string;
  entryX: string;
  entryY: string;
}

export const DEFAULT_FLOOR: DungeonFloor = {
  regenFile: "",
  bossVnum: "",
  entryX: "",
  entryY: "",
};

export interface DungeonFormState {
  entryNpcVnum: string;
  entryChatLabel: string;
  entryTitle: string;
  entryText: string;
  requiredLevel: string;
  dungeonMapIndex: string;
  mapIndexRangeStart: string;
  mapIndexRangeEnd: string;
  exitDelaySeconds: string;
  rewardItemVnum: string;
  rewardItemCount: string;
  rewardMoney: string;
  floors: DungeonFloor[];
}

export const DEFAULT_DUNGEON_FORM: DungeonFormState = {
  entryNpcVnum: "",
  entryChatLabel: "Run",
  entryTitle: "",
  entryText: "",
  requiredLevel: "0",
  dungeonMapIndex: "",
  mapIndexRangeStart: "",
  mapIndexRangeEnd: "",
  exitDelaySeconds: "30",
  rewardItemVnum: "",
  rewardItemCount: "1",
  rewardMoney: "",
  floors: [{ ...DEFAULT_FLOOR }],
};

function inRangeGuard(form: DungeonFormState): string {
  return `pc.get_map_index() >= ${Number(form.mapIndexRangeStart) || 0} and pc.get_map_index() < ${Number(form.mapIndexRangeEnd) || 0}`;
}

export function generateDungeonQuest(questName: string, form: DungeonFormState): string {
  const npcVnum = Number(form.entryNpcVnum) || 0;
  const dungeonMap = Number(form.dungeonMapIndex) || 0;
  const requiredLevel = Number(form.requiredLevel) || 0;
  const exitDelay = Math.max(0, Number(form.exitDelaySeconds) || 0);
  const floors = form.floors.length > 0 ? form.floors : [DEFAULT_FLOOR];

  const lines: string[] = [];
  lines.push(`quest ${questName} begin`);
  lines.push(`\tstate start begin`);

  lines.push(`\t\t-- Einstieg: NPC-Dialog warpt auf die Dungeon-Map.`);
  const entryBody = [
    ...(requiredLevel > 0
      ? [
          `if pc.get_level() < ${requiredLevel} then`,
          `\tsay_title(${luaString(form.entryTitle)})`,
          `\tsay("Du bist noch nicht stark genug für diesen Run.")`,
          `\treturn`,
          `end`,
        ]
      : []),
    `say_title(${luaString(form.entryTitle)})`,
    `say(${luaString(form.entryText)})`,
    `local enter = select("Betreten", "Abbrechen")`,
    `if enter == 1 then`,
    `\tpc.warp(${Number(floors[0].entryX) || 0} * 100, ${Number(floors[0].entryY) || 0} * 100, ${dungeonMap})`,
    `end`,
  ];
  lines.push(`\t\twhen ${npcVnum}.chat.${luaString(form.entryChatLabel)} begin`);
  lines.push(indent(entryBody, 3));
  lines.push(`\t\tend`);
  lines.push("");

  lines.push(`\t\t-- Ankunft in der Instanz: Etage 1 vorbereiten.`);
  lines.push(`\t\twhen enter begin`);
  lines.push(
    indent(
      [
        `if ${inRangeGuard(form)} then`,
        `\td.setf("level", 1)`,
        `\td.regen_file(${luaString(floors[0].regenFile)})`,
        `end`,
      ],
      3,
    ),
  );
  lines.push(`\t\tend`);
  lines.push("");

  floors.forEach((floor, index) => {
    const level = index + 1;
    const isLast = index === floors.length - 1;
    const bossVnum = Number(floor.bossVnum) || 0;
    lines.push(`\t\t-- Etage ${level}: ${isLast ? "Boss besiegen -> Run abschließen." : "Boss besiegen -> nächste Etage."}`);
    lines.push(
      `\t\twhen ${bossVnum}.kill with pc.in_dungeon() and ${inRangeGuard(form)} and d.getf("level") == ${level} begin`,
    );

    const body: string[] = [`d.clear_regen()`];
    if (!isLast) {
      const next = floors[index + 1];
      body.push(
        `d.setf("level", ${level + 1})`,
        `d.jump_all(${Number(next.entryX) || 0}, ${Number(next.entryY) || 0})`,
        `d.regen_file(${luaString(next.regenFile)})`,
      );
    } else {
      const rewardVnum = Number(form.rewardItemVnum);
      if (Number.isFinite(rewardVnum) && rewardVnum > 0) {
        const count = Math.max(1, Number(form.rewardItemCount) || 1);
        body.push(`game.drop_item_with_ownership(${rewardVnum}, ${count})`);
      }
      const money = Number(form.rewardMoney);
      if (Number.isFinite(money) && money !== 0) {
        body.push(`pc.changemoney(${money})`);
      }
      body.push(
        `notice_multiline("Der Run wurde erfolgreich abgeschlossen!", d.notice)`,
        `server_timer("${questName}_exit", ${exitDelay}, d.get_map_index())`,
      );
    }
    lines.push(indent(body, 3));
    lines.push(`\t\tend`);
    lines.push("");
  });

  lines.push(`\t\t-- Nach der Belohnung alle Spieler aus der Instanz werfen.`);
  lines.push(`\t\twhen ${questName}_exit.server_timer begin`);
  lines.push(
    indent(
      [
        `if d.select(get_server_timer_arg()) then`,
        `\td.exit_all()`,
        `end`,
      ],
      3,
    ),
  );
  lines.push(`\t\tend`);

  lines.push(`\tend`);
  lines.push(`end`);
  lines.push("");
  return lines.join("\n");
}

// ---- Mehrschritt-Vorlage ----
//
// Verkettet mehrere der bekannten Bausteine (Dialog/Sammeln/Töten/Item
// benutzen) zu einer echten Mehrschritt-Quest - jeder Schritt ist ein
// eigener `state <name> begin...end`-Block, der Übergang läuft über
// `set_state(<name>)`. Diese Mechanik stand ursprünglich nur im 1:1
// kopierten Community-Wiki (core-concepts.md) und wurde deshalb zunächst
// bewusst vermieden - inzwischen direkt im echten Server-Quellcode dieses
// Projekts verifiziert: `set_state`/`setstate` sind in
// game-src/source/game/src/questlua_quest.cpp an `quest_setstate`
// gebunden, das über `CQuestManager::GetQuestStateIndex` den Zustandsnamen
// auflöst und `PC::SetQuestState` (questpc.cpp) aufruft - exakt der vom
// Wiki beschriebene Mechanismus. `__COMPLETE__` ist dabei keine
// Sonderkonstante der Engine, sondern reine Namenskonvention: ein
// gewöhnlicher (leerer) Zustand, den `GetQuestStateIndex` genauso wie
// jeden anderen per Lua-Tabellen-Lookup auflöst.
//
// Wichtig, ebenfalls quellcode-verifiziert: `pc.getqf`/`pc.setqf` sind
// NICHT pro Zustand gescoped, sondern global pro Quest (Schlüssel
// `<questName>.<flag>`, siehe questlua_pc.cpp::pc_get/set_quest_flag).
// Ein Kill-Zähler braucht deshalb weiterhin einen pro Schritt eigenen,
// namensraum-isolierten qf-Key (kill_count_stepN) - sonst würden zwei
// Kill-Schritte in derselben Quest denselben Zähler teilen.

export type StepKind = "dialog" | "collect" | "kill" | "use";

export interface QuestStep {
  kind: StepKind;
  npcVnum: string;
  chatLabel: string;
  npcTitle: string;
  dialogText: string; // dialog
  requiredItemVnum: string;
  requiredItemCount: string;
  successText: string;
  failText: string; // collect
  mobVnum: string;
  requiredKills: string;
  progressText: string; // kill
  useItemVnum: string;
  useText: string; // use
  // Optionale Zwischenbelohnung - bei jeder Step-Art nutzbar, nicht nur beim
  // letzten Schritt (z.B. "hier schon mal etwas Yang, mach weiter").
  rewardItemVnum: string;
  rewardItemCount: string;
  rewardMoney: string;
  // Feste Bonus-Attribute auf dem Belohnungs-Item (z.B. "Max. HP +250"),
  // exakt derselbe Mechanismus wie bei der buffed_item-Vorlage weiter oben
  // (pc.give_item2_select + item.set_value statt pc.give_item2) - "0"/
  // Apply-Typ "Keine" bedeutet der Slot bleibt ungenutzt.
  rewardAttrType0: string;
  rewardAttrValue0: string;
  rewardAttrType1: string;
  rewardAttrValue1: string;
  rewardAttrType2: string;
  rewardAttrValue2: string;
  rewardAttrType3: string;
  rewardAttrValue3: string;
}

export const DEFAULT_STEP: QuestStep = {
  kind: "dialog",
  npcVnum: "",
  chatLabel: "Quest",
  npcTitle: "",
  dialogText: "",
  requiredItemVnum: "",
  requiredItemCount: "1",
  successText: "",
  failText: "Das reicht noch nicht.",
  mobVnum: "",
  requiredKills: "10",
  progressText: "Du hast %d von %d erledigt.",
  useItemVnum: "",
  useText: "",
  rewardItemVnum: "",
  rewardItemCount: "1",
  rewardMoney: "",
  rewardAttrType0: "0",
  rewardAttrValue0: "0",
  rewardAttrType1: "0",
  rewardAttrValue1: "0",
  rewardAttrType2: "0",
  rewardAttrValue2: "0",
  rewardAttrType3: "0",
  rewardAttrValue3: "0",
};

export interface MultiStepFormState {
  steps: QuestStep[];
  // Wiederholbare Quest mit Cooldown (z.B. "alle 4 Tage") statt einmalig -
  // siehe generateMultiStepQuest für die Cooldown-Sperre an Schritt 0 und
  // den Kill-Zähler-Reset beim erneuten Durchlauf.
  repeatable: boolean;
  cooldownDays: string;
}

export const DEFAULT_MULTI_STEP_FORM: MultiStepFormState = {
  steps: [{ ...DEFAULT_STEP }],
  repeatable: false,
  cooldownDays: "4",
};

export const STEP_LABELS: Record<StepKind, string> = {
  dialog: "NPC-Dialog (mit optionaler Zwischenbelohnung)",
  collect: "Items abgeben",
  kill: "Monster töten",
  use: "Item benutzen",
};

// Zustandsname pro Schritt: der erste Schritt ist immer "start" (von der
// Engine als Startzustand vorausgesetzt), alle weiteren heißen "step_N"
// (N = Schritt-Nummer, 1-basiert) - passend zu den "Schritt N"-Kartenlabeln
// in der UI (Schritt 2 -> state step_2 usw.).
function stateName(index: number): string {
  return index === 0 ? "start" : `step_${index + 1}`;
}

// Bei einer wiederholbaren Quest gibt es keinen echten Abschluss-Zustand -
// der letzte Schritt springt zurück auf "start" (Kreis statt Sackgasse),
// die Cooldown-Sperre an Schritt 0 verhindert einen sofortigen Neustart.
function nextStateName(index: number, isLast: boolean, repeatable: boolean): string {
  if (isLast) return repeatable ? "start" : "__COMPLETE__";
  return stateName(index + 1);
}

function stepRewardLines(step: QuestStep): string[] {
  const lines: string[] = [];
  const itemVnum = Number(step.rewardItemVnum);
  const attrSlots: [string, string][] = [
    [step.rewardAttrType0, step.rewardAttrValue0],
    [step.rewardAttrType1, step.rewardAttrValue1],
    [step.rewardAttrType2, step.rewardAttrValue2],
    [step.rewardAttrType3, step.rewardAttrValue3],
  ];
  const hasAttrs = attrSlots.some(([type]) => (Number(type) || 0) > 0);

  if (Number.isFinite(itemVnum) && itemVnum > 0) {
    const count = Math.max(1, Number(step.rewardItemCount) || 1);
    if (hasAttrs) {
      // Fest eingebaute Bonus-Attribute (z.B. "Max. HP +250") - derselbe
      // Mechanismus wie buffedItemTemplate weiter oben: give_item2_select
      // markiert das neue Item als "aktuelles Item", item.set_value
      // schreibt direkt in einen der 7 Bonus-Slots statt eines
      // Zufalls-Rolls.
      lines.push(`pc.give_item2_select(${itemVnum}, ${count})`);
      attrSlots.forEach(([type, value], slot) => {
        const applyType = Number(type) || 0;
        const applyValue = Number(value) || 0;
        if (applyType > 0) lines.push(`item.set_value(${slot}, ${applyType}, ${applyValue})`);
      });
    } else {
      lines.push(`pc.give_item2(${itemVnum}, ${count})`);
    }
  }
  const money = Number(step.rewardMoney);
  if (Number.isFinite(money) && money !== 0) {
    lines.push(`pc.changemoney(${money})`);
  }
  return lines;
}

// Cooldown-Bedingung für den Eintritt in Schritt 0 einer wiederholbaren
// Quest - get_time()/get_global_time() ist eine echte, registrierte
// Quest-Funktion (game-src/source/game/src/questlua_global.cpp), nicht
// aus dem Wiki übernommen. "not (...)" negiert dieselbe Bedingung für den
// "noch nicht wieder verfügbar"-Geschwisterblock, statt eine zweite
// Formel von Hand gegenzurechnen (Fehlerquelle vermieden).
function cooldownGuardExpr(days: number): string {
  return `pc.getqf("last_complete") == 0 or get_time() - pc.getqf("last_complete") >= ${Math.max(0, days) * 86400}`;
}

function cooldownWaitBlock(trigger: string, cooldownGuard: string): string[] {
  return [
    `when ${trigger} with not (${cooldownGuard}) begin`,
    indent([`say(${luaString("Diese Quest ist noch nicht wieder verfügbar.")})`], 1),
    `end`,
  ];
}

type StepLineGenerator = (
  step: QuestStep,
  index: number,
  next: string,
  completionExtra: string[],
  cooldownGuard: string | null,
) => string[];

const dialogStepLines: StepLineGenerator = (step, _index, next, completionExtra, cooldownGuard) => {
  const npcVnum = Number(step.npcVnum) || 0;
  const trigger = `${npcVnum}.chat.${luaString(step.chatLabel)}`;
  const body = [
    `say_title(${luaString(step.npcTitle)})`,
    `say(${luaString(step.dialogText)})`,
    ...stepRewardLines(step),
    ...completionExtra,
    `set_state(${next})`,
  ];
  const lines = [
    `when ${trigger}${cooldownGuard ? ` with ${cooldownGuard}` : ""} begin`,
    indent(body, 1),
    `end`,
  ];
  return cooldownGuard ? [...lines, ...cooldownWaitBlock(trigger, cooldownGuard)] : lines;
};

const collectStepLines: StepLineGenerator = (step, _index, next, completionExtra, cooldownGuard) => {
  const npcVnum = Number(step.npcVnum) || 0;
  const requiredVnum = Number(step.requiredItemVnum) || 0;
  const requiredCount = Math.max(1, Number(step.requiredItemCount) || 1);
  const trigger = `${npcVnum}.chat.${luaString(step.chatLabel)}`;
  const successBody = [
    `say_title(${luaString(step.npcTitle)})`,
    `say(${luaString(step.successText)})`,
    `pc.remove_item(${requiredVnum}, ${requiredCount})`,
    ...stepRewardLines(step),
    ...completionExtra,
    `set_state(${next})`,
  ];
  const failBody = [
    `say_title(${luaString(step.npcTitle)})`,
    `say(${luaString(step.failText)})`,
  ];
  const lines = [
    `when ${trigger}${cooldownGuard ? ` with ${cooldownGuard}` : ""} begin`,
    `\tif pc.count_item(${requiredVnum}) >= ${requiredCount} then`,
    indent(successBody, 2),
    `\telse`,
    indent(failBody, 2),
    `\tend`,
    `end`,
  ];
  return cooldownGuard ? [...lines, ...cooldownWaitBlock(trigger, cooldownGuard)] : lines;
};

// Eigener, nach Schritt-Index benannter qf-Key (z.B. "kill_count_step2") -
// pc.getqf/setqf sind NICHT pro Zustand gescoped (quellcode-verifiziert,
// siehe Kommentar oben), sonst würden zwei Kill-Schritte in derselben
// Quest denselben Zähler teilen und sich gegenseitig kaputt machen. Bei
// einer wiederholbaren Quest wird dieser Zähler zusätzlich bei jedem
// Abschluss zurückgesetzt (siehe generateMultiStepQuest).
const killStepLines: StepLineGenerator = (step, index, next, completionExtra, cooldownGuard) => {
  const npcVnum = Number(step.npcVnum) || 0;
  const mobVnum = Number(step.mobVnum) || 0;
  const requiredKills = Math.max(1, Number(step.requiredKills) || 1);
  const qfKey = `kill_count_step${index}`;
  const trigger = `${npcVnum}.chat.${luaString(step.chatLabel)}`;
  const progressBody = [
    `say_title(${luaString(step.npcTitle)})`,
    `say(string.format(${luaString(step.progressText)}, pc.getqf(${luaString(qfKey)}), ${requiredKills}))`,
  ];
  const successBody = [
    `say_title(${luaString(step.npcTitle)})`,
    `say(${luaString(step.successText)})`,
    ...stepRewardLines(step),
    ...completionExtra,
    `set_state(${next})`,
  ];
  const guardSuffix = cooldownGuard ? ` and (${cooldownGuard})` : "";
  const lines = [
    `when ${trigger} with pc.getqf(${luaString(qfKey)}) < ${requiredKills}${guardSuffix} begin`,
    indent(progressBody, 1),
    `end`,
    `when ${trigger} with pc.getqf(${luaString(qfKey)}) >= ${requiredKills}${guardSuffix} begin`,
    indent(successBody, 1),
    `end`,
    `when ${mobVnum}.kill begin`,
    `\tif pc.getqf(${luaString(qfKey)}) < ${requiredKills} then`,
    `\t\tpc.setqf(${luaString(qfKey)}, pc.getqf(${luaString(qfKey)}) + 1)`,
    `\tend`,
    `end`,
  ];
  return cooldownGuard ? [...lines, ...cooldownWaitBlock(trigger, cooldownGuard)] : lines;
};

const useStepLines: StepLineGenerator = (step, _index, next, completionExtra, cooldownGuard) => {
  const useItemVnum = Number(step.useItemVnum) || 0;
  const trigger = `${useItemVnum}.use`;
  const body = [
    `say(${luaString(step.useText)})`,
    ...stepRewardLines(step),
    ...completionExtra,
    `set_state(${next})`,
  ];
  const lines = [
    `when ${trigger}${cooldownGuard ? ` with ${cooldownGuard}` : ""} begin`,
    indent(body, 1),
    `end`,
  ];
  return cooldownGuard ? [...lines, ...cooldownWaitBlock(trigger, cooldownGuard)] : lines;
};

const STEP_GENERATORS: Record<StepKind, StepLineGenerator> = {
  dialog: dialogStepLines,
  collect: collectStepLines,
  kill: killStepLines,
  use: useStepLines,
};

export function generateMultiStepQuest(questName: string, form: MultiStepFormState): string {
  const steps = form.steps.length > 0 ? form.steps : [DEFAULT_STEP];
  const cooldownDays = Math.max(0, Number(form.cooldownDays) || 0);
  const cooldownGuard = form.repeatable ? cooldownGuardExpr(cooldownDays) : null;

  const lines: string[] = [`quest ${questName} begin`];
  steps.forEach((step, index) => {
    const isLast = index === steps.length - 1;
    const next = nextStateName(index, isLast, form.repeatable);
    const completionExtra: string[] = [];
    if (isLast && form.repeatable) {
      completionExtra.push(`pc.setqf("last_complete", get_time())`);
      steps.forEach((s, i) => {
        if (s.kind === "kill") completionExtra.push(`pc.setqf(${luaString(`kill_count_step${i}`)}, 0)`);
      });
    }
    lines.push(`\t-- Schritt ${index + 1}${isLast ? " (Abschluss)" : ""}`);
    lines.push(`\tstate ${stateName(index)} begin`);
    lines.push(
      indent(
        STEP_GENERATORS[step.kind](step, index, next, completionExtra, index === 0 ? cooldownGuard : null),
        2,
      ),
    );
    lines.push(`\tend`);
    lines.push("");
  });
  // Terminaler Zustand - reine Namenskonvention (quellcode-verifiziert,
  // keine Sonderbehandlung durch die Engine), muss aber existieren, damit
  // set_state("__COMPLETE__") einen gültigen Zustand auflöst. Bei einer
  // wiederholbaren Quest wird er nie erreicht (letzter Schritt springt
  // stattdessen auf "start" zurück) - deshalb hier weggelassen.
  if (!form.repeatable) {
    lines.push(`\tstate __COMPLETE__ begin`);
    lines.push(`\tend`);
  }
  lines.push(`end`);
  lines.push("");
  return lines.join("\n");
}
