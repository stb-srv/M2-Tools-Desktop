import type { MobDropGroup } from "./types";

export interface DuplicateItemFinding {
  groupIndex: number;
  mobName: string;
  mobVnum: number;
  itemVnum: number;
  itemIndices: number[];
}

export interface DuplicateMobFinding {
  mobVnum: number;
  groupIndices: number[];
}

// Innerhalb eines Mobs dieselbe Item-VNUM mehrfach in der Drop-Liste -
// passiert leicht beim manuellen Bearbeiten der mob_drop_item.txt.
// itemIndices enthält jede Position, damit eine einzelne Dopplung gezielt
// entfernt werden kann statt alle auf einmal.
export function findDuplicateItemsInMobs(groups: MobDropGroup[]): DuplicateItemFinding[] {
  const findings: DuplicateItemFinding[] = [];
  groups.forEach((g, groupIndex) => {
    const byVnum = new Map<number, number[]>();
    g.items.forEach((item, itemIndex) => {
      const list = byVnum.get(item.item_vnum) ?? [];
      list.push(itemIndex);
      byVnum.set(item.item_vnum, list);
    });
    byVnum.forEach((itemIndices, itemVnum) => {
      if (itemIndices.length > 1) {
        findings.push({ groupIndex, mobName: g.name, mobVnum: g.mob_vnum, itemVnum, itemIndices });
      }
    });
  });
  return findings;
}

// Derselbe Mob (gleiche mob_vnum) kommt mehrfach als eigener Eintrag in der
// Datei vor - unabhängig von der obigen Prüfung, ein Mob kann in beiden
// Listen auftauchen.
export function findDuplicateMobs(groups: MobDropGroup[]): DuplicateMobFinding[] {
  const byVnum = new Map<number, number[]>();
  groups.forEach((g, groupIndex) => {
    const list = byVnum.get(g.mob_vnum) ?? [];
    list.push(groupIndex);
    byVnum.set(g.mob_vnum, list);
  });
  const findings: DuplicateMobFinding[] = [];
  byVnum.forEach((groupIndices, mobVnum) => {
    if (groupIndices.length > 1) findings.push({ mobVnum, groupIndices });
  });
  return findings;
}
