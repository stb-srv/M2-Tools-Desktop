export interface ColumnInfo {
  name: string;
  data_type: string;
  is_nullable: boolean;
  is_primary_key: boolean;
}

export interface TableRows {
  columns: string[];
  rows: (string | null)[][];
  total_rows: number;
}

export interface TableTarget {
  database: string;
  table: string;
}

export interface AccountSummary {
  id: number;
  login: string;
  email: string;
  status: string;
  empire: number;
  create_time: string;
  last_play: string | null;
}

export interface BanRecord {
  id: number;
  account_id: number;
  login: string;
  reason: string;
  banned_at: string;
  unban_at: string | null;
  active: boolean;
}

// Verified table locations from earlier live-DB introspection (see
// [[m2manager-db-schema]] memory) - account/player data lives in these two
// tables on this core. The search column and primary key are NOT assumed
// beyond that: search column is user-editable (schemas vary in which column
// holds the login/character name), and the primary key is auto-detected at
// runtime by GenericRowEditor via `is_primary_key`, never guessed as "id".
export const PLAYER_TABLE: TableTarget = { database: "player", table: "player" };
export const ITEM_TABLE: TableTarget = { database: "player", table: "item" };
