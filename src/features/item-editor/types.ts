export interface ItemProtoInput {
  vnum: number;
  vnum_range: number;
  name: string;
  locale_name: string;
  type: number;
  subtype: number;
  weight: number;
  size: number;
  antiflag: number;
  flag: number;
  wearflag: number;
  immuneflag: number;
  gold: number;
  shop_buy_price: number;
  refined_vnum: number;
  refine_set: number;
  magic_pct: number;
  limittype0: number;
  limitvalue0: number;
  limittype1: number;
  limitvalue1: number;
  applytype0: number;
  applyvalue0: number;
  applytype1: number;
  applyvalue1: number;
  applytype2: number;
  applyvalue2: number;
  applytype3: number;
  applyvalue3: number;
  value0: number;
  value1: number;
  value2: number;
  value3: number;
  value4: number;
  value5: number;
  socket0: number;
  socket1: number;
  socket2: number;
  socket3: number;
  socket4: number;
  socket5: number;
  specular: number;
  socket_pct: number;
  addon_type: number;
}

export interface ItemDescEntry {
  vnum: number;
  description: string;
  summary: string;
  extra: string | null;
}

export type StepStatus = "pending" | "running" | "done" | "error";
export type Mode = "create" | "edit";

export function emptyItem(vnum: number): ItemProtoInput {
  return {
    vnum,
    vnum_range: 0,
    name: "",
    locale_name: "",
    type: 3,
    subtype: 0,
    weight: 0,
    size: 1,
    antiflag: 0,
    flag: 0,
    wearflag: 0,
    immuneflag: 0,
    gold: 0,
    shop_buy_price: 0,
    refined_vnum: 0,
    refine_set: 0,
    magic_pct: 0,
    limittype0: 0,
    limitvalue0: 0,
    limittype1: 0,
    limitvalue1: 0,
    applytype0: 0,
    applyvalue0: 0,
    applytype1: 0,
    applyvalue1: 0,
    applytype2: 0,
    applyvalue2: 0,
    applytype3: 0,
    applyvalue3: 0,
    value0: 0,
    value1: 0,
    value2: 0,
    value3: 0,
    value4: 0,
    value5: 0,
    socket0: 0,
    socket1: 0,
    socket2: 0,
    socket3: 0,
    socket4: 0,
    socket5: 0,
    specular: 0,
    socket_pct: 0,
    addon_type: 0,
  };
}
