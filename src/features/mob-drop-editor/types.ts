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

// See mobdrop::check_numbering (Rust) - the leading index on each item line
// in mob_drop_item.txt isn't cosmetic: the real server loader looks items up
// by that literal string key ("1", "2", ...) and stops at the first missing
// key, silently dropping every entry after a gap. `found` is the raw index
// values as they appear in the file, in file order.
export interface NumberingIssue {
  group_index: number;
  found: number[];
}

export interface MobDropLoadResult {
  groups: MobDropGroup[];
  numbering_issues: NumberingIssue[];
}

export interface ItemSearchResult {
  vnum: number;
  name: string;
}

export type BulkMode = "delta" | "fixed" | "random" | "specific-item";
export type BulkScope = "global" | "current";
