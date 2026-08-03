// Lua generators for the Quest Builder's "template" mode. Every construct
// used here (when NPC.chat."label" [with cond] begin...end, when MOB.kill
// begin...end, when ITEM.use begin...end, say/say_title, pc.give_item2,
// pc.changemoney, pc.count_item/remove_item, pc.getqf/setqf, string.format)
// was verified against real quest source files on the user's dev server
// (Pferd/Pferd_rufen.lua, Biologie/Orkzahn.lua) - not guessed from generic
// Metin2 documentation. Templates only cover the common single-state cases;
// anything more complex is edited as raw code after creation.

export type TemplateType = "dialog" | "collect" | "chance_collect" | "kill" | "use" | "dungeon";

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

type SimpleTemplateType = Exclude<TemplateType, "dungeon">;

const TEMPLATE_BODIES: Record<SimpleTemplateType, (form: QuestFormState) => string> = {
  dialog: dialogTemplate,
  collect: collectTemplate,
  chance_collect: chanceCollectTemplate,
  kill: killTemplate,
  use: useTemplate,
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
  dungeon: "Run / Instanz-Dungeon (mehrere Etagen)",
};

export const TEMPLATE_HINTS: Record<TemplateType, string> = {
  dialog: "Spieler spricht den NPC an und bekommt direkt eine Belohnung.",
  collect:
    "Spieler muss eine bestimmte Anzahl eines Items besitzen und beim NPC abgeben, um die Belohnung zu erhalten.",
  chance_collect:
    "Wie bei den echten Biologie-Quests (z.B. Orkzahn.lua): Spieler gibt bei jedem Versuch ein Item ab, das Item wird IMMER verbraucht, zählt aber nur mit einer festgelegten Chance zum Fortschritt. Erst nach genug Erfolgen gibt es die Belohnung.",
  kill: "Spieler muss eine bestimmte Anzahl eines Monsters töten (Fortschritt wird pro Spieler gespeichert), dann beim NPC melden.",
  use: "Ein Item löst beim Benutzen direkt einen Effekt/eine Belohnung aus (kein NPC nötig).",
  dungeon:
    "Grundgerüst für einen mehrstufigen Run wie Beran/Daemonenturm/Teufelskatakomben: NPC-Einstieg, pro Etage ein Boss-Kill schaltet die nächste Etage frei (d.regen_file/d.jump_all/d.setf), am Ende Belohnung + Rauswurf. Deckt bewusst nur das gemeinsame Grundmuster ab - Passwort-Systeme, Zeitlimits o.ä. wie beim echten Beran müssen danach im Code-Editor von Hand ergänzt werden.",
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
