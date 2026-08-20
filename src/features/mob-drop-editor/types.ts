export interface MobDropItem {
  item_vnum: number;
  count: number;
  percent: number;
}

export interface MobDropGroup {
  name: string;
  mob_vnum: number;
  drop_type: string;
  items: MobDropItem[];
}

export interface ItemSearchResult {
  vnum: number;
  name: string;
}

export type BulkMode = "delta" | "fixed" | "random" | "specific-item";
export type BulkScope = "global" | "current";
