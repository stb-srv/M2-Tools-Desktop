import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { PasswordInput } from "@/components/ui/password-input";
import { autoConnectMysql } from "@/lib/mysqlConnect";
import { Gr2Canvas } from "@/features/model-viewer/Gr2Canvas";
import type { Gr2ModelInfo } from "@/features/model-viewer/types";
import { Minus, Plus, Search, Trash2, AlertTriangle, X, RefreshCw, HelpCircle } from "lucide-react";
import { openManual } from "@/lib/manual";

interface ShopSummary {
  vnum: number;
  name: string;
  npc_vnum: number;
  npc_name: string;
  npc_folder: string | null;
  item_count: number;
}

interface ShopItem {
  item_vnum: number;
  item_name: string;
  count: number;
  size: number;
}

interface ItemSearchResult {
  vnum: number;
  name: string;
}

type CellSelection =
  | { kind: "empty"; index: number }
  | { kind: "filled"; index: number; item: ShopItem };

interface PlacedItem extends ShopItem {
  col: number;
  row: number;
  span: number;
}

// Items are 1 slot wide but `size` slots TALL (item_proto.size, 1-3), so a
// size-3 item blocks the two cells below it - same as the in-game grid.
// Place them top-left first, then expose whatever cells stay free as the
// clickable "add item here" slots.
function placeItems(items: ShopItem[], columns: number, rows: number) {
  const occupied = new Set<string>();
  const placedItems: PlacedItem[] = [];
  const overflowItems: ShopItem[] = [];

  for (const item of items) {
    const span = Math.min(Math.max(item.size, 1), rows);
    let placed = false;

    for (let row = 1; row <= rows - span + 1 && !placed; row++) {
      for (let col = 1; col <= columns && !placed; col++) {
        const cells = Array.from({ length: span }, (_, i) => `${row + i},${col}`);
        if (cells.some((c) => occupied.has(c))) continue;
        cells.forEach((c) => occupied.add(c));
        placedItems.push({ ...item, col, row, span });
        placed = true;
      }
    }

    if (!placed) overflowItems.push(item);
  }

  const emptyCells: { row: number; col: number }[] = [];
  for (let row = 1; row <= rows; row++) {
    for (let col = 1; col <= columns; col++) {
      if (!occupied.has(`${row},${col}`)) emptyCells.push({ row, col });
    }
  }

  return { placedItems, emptyCells, overflowItems, usedSlots: occupied.size };
}

export function ShopEditor() {
  const { t } = useTranslation();

  const [connected, setConnected] = useState(false);
  const [checkingConnection, setCheckingConnection] = useState(true);
  const [connecting, setConnecting] = useState(false);
  const [connError, setConnError] = useState<string | null>(null);
  const [host, setHost] = useState("");
  const [port, setPort] = useState("3306");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");

  const [shops, setShops] = useState<ShopSummary[]>([]);
  const [shopSearch, setShopSearch] = useState("");
  const [selectedShop, setSelectedShop] = useState<ShopSummary | null>(null);
  const [shopItems, setShopItems] = useState<ShopItem[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [renaming, setRenaming] = useState(false);
  const [nameDraft, setNameDraft] = useState("");

  // Metin2 shop windows hold at most 40 slots - columns/rows are cosmetic
  // grid layout, but their product must never exceed that hard game limit.
  const MAX_SLOTS = 40;
  const [columns, setColumns] = useState(5);
  const [rows, setRows] = useState(8);
  const [mode, setMode] = useState<"global" | "pro-shop">("global");
  const [maxValue, setMaxValue] = useState(200);
  const [icons, setIcons] = useState<Record<number, string | null>>({});

  const [npcModel, setNpcModel] = useState<Gr2ModelInfo | null>(null);
  const [npcModelError, setNpcModelError] = useState<string | null>(null);
  const [npcModelLoading, setNpcModelLoading] = useState(false);

  const [cell, setCell] = useState<CellSelection | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<ItemSearchResult[]>([]);
  const [searching, setSearching] = useState(false);

  const [syncConfirm, setSyncConfirm] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [syncResult, setSyncResult] = useState<number | null>(null);

  const [deleteConfirm, setDeleteConfirm] = useState<ShopSummary | null>(null);
  const [creating, setCreating] = useState(false);
  const [newShopName, setNewShopName] = useState("");
  const [newShopNpcVnum, setNewShopNpcVnum] = useState("");

  useEffect(() => {
    (async () => {
      try {
        // Usually already connected by the time this mounts - App.tsx
        // auto-connects at startup. This just covers the case where that
        // failed (e.g. no stored credentials yet) by prefilling the manual
        // form, and re-checks in case a connection was made since.
        const connected = await autoConnectMysql();
        if (connected) {
          setConnected(true);
          await refreshShops();
          return;
        }
        const [savedHost, savedPort, savedUser] = await Promise.all([
          invoke<string | null>("get_setting", { key: "mysql_host" }),
          invoke<string | null>("get_setting", { key: "mysql_port" }),
          invoke<string | null>("get_setting", { key: "mysql_username" }),
        ]);
        if (savedHost) setHost(savedHost);
        if (savedPort) setPort(savedPort);
        if (savedUser) setUsername(savedUser);
      } catch {
        // fall through to manual connect form
      } finally {
        setCheckingConnection(false);
      }
    })();
  }, []);

  useEffect(() => {
    if (!selectedShop) {
      setNpcModel(null);
      setNpcModelError(null);
      return;
    }
    let cancelled = false;
    setNpcModelLoading(true);
    setNpcModelError(null);
    setNpcModel(null);
    (async () => {
      try {
        const [dllPath, gr2Path] = await invoke<[string, string]>("locate_npc_model", {
          npcVnum: selectedShop.npc_vnum,
          folder: selectedShop.npc_folder,
        });
        const model = await invoke<Gr2ModelInfo>("load_gr2_model", {
          grannyDllPath: dllPath,
          gr2Path,
        });
        if (!cancelled) setNpcModel(model);
      } catch (e) {
        if (!cancelled) setNpcModelError(String(e));
      } finally {
        if (!cancelled) setNpcModelLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [selectedShop?.vnum, selectedShop?.npc_vnum, selectedShop?.npc_folder]);

  useEffect(() => {
    if (!selectedShop && mode === "pro-shop") return;
    invoke<number>("get_shop_default_max", {
      shopVnum: mode === "pro-shop" ? (selectedShop?.vnum ?? null) : null,
    })
      .then(setMaxValue)
      .catch(() => {});
  }, [mode, selectedShop?.vnum]);

  function ensureIcons(vnums: number[]) {
    const missing = [...new Set(vnums)].filter((v) => !(v in icons));
    if (missing.length === 0) return;
    missing.forEach((vnum) => {
      invoke<string | null>("get_item_icon", { vnum })
        .then((dataUrl) => setIcons((prev) => ({ ...prev, [vnum]: dataUrl })))
        .catch(() => setIcons((prev) => ({ ...prev, [vnum]: null })));
    });
  }

  useEffect(() => {
    ensureIcons(shopItems.map((i) => i.item_vnum));
  }, [shopItems]);

  useEffect(() => {
    ensureIcons(searchResults.map((r) => r.vnum));
  }, [searchResults]);

  function updateColumns(next: number) {
    const clamped = Math.max(1, Math.min(MAX_SLOTS, next));
    const newRows = clamped * rows > MAX_SLOTS ? Math.max(1, Math.floor(MAX_SLOTS / clamped)) : rows;
    setColumns(clamped);
    setRows(newRows);
  }

  function updateRows(next: number) {
    const clamped = Math.max(1, Math.min(MAX_SLOTS, next));
    const newCols =
      columns * clamped > MAX_SLOTS ? Math.max(1, Math.floor(MAX_SLOTS / clamped)) : columns;
    setRows(clamped);
    setColumns(newCols);
  }

  async function handleConnect() {
    setConnecting(true);
    setConnError(null);
    try {
      const config = { host, port: Number(port), username, database: null };
      await invoke("connect_mysql", { config, password });
      await invoke("set_setting", { key: "mysql_host", value: host });
      await invoke("set_setting", { key: "mysql_port", value: port });
      await invoke("set_setting", { key: "mysql_username", value: username });
      await invoke("store_credential", { account: "mysql_password", secret: password });
      setConnected(true);
      await refreshShops();
    } catch (e) {
      setConnError(String(e));
    } finally {
      setConnecting(false);
    }
  }

  async function refreshShops() {
    setError(null);
    try {
      const result = await invoke<ShopSummary[]>("list_shops");
      setShops(result);
    } catch (e) {
      setError(String(e));
    }
  }

  async function selectShop(shop: ShopSummary) {
    setSelectedShop(shop);
    setRenaming(false);
    setError(null);
    try {
      const items = await invoke<ShopItem[]>("get_shop_items", { shopVnum: shop.vnum });
      setShopItems(items);
    } catch (e) {
      setError(String(e));
    }
  }

  async function changeCount(item: ShopItem, delta: number) {
    if (!selectedShop) return;
    const nextCount = Math.max(1, item.count + delta);
    setShopItems((prev) =>
      prev.map((i) => (i.item_vnum === item.item_vnum ? { ...i, count: nextCount } : i)),
    );
    try {
      await invoke("update_shop_item_count", {
        shopVnum: selectedShop.vnum,
        itemVnum: item.item_vnum,
        count: nextCount,
      });
    } catch (e) {
      setError(String(e));
    }
  }

  async function removeItem(item: ShopItem) {
    if (!selectedShop) return;
    try {
      await invoke("remove_shop_item", {
        shopVnum: selectedShop.vnum,
        itemVnum: item.item_vnum,
      });
      setShopItems((prev) => prev.filter((i) => i.item_vnum !== item.item_vnum));
      setCell(null);
    } catch (e) {
      setError(String(e));
    }
  }

  async function addItem(item: ItemSearchResult) {
    if (!selectedShop) return;
    try {
      await invoke("add_shop_item", {
        shopVnum: selectedShop.vnum,
        itemVnum: item.vnum,
        count: maxValue,
      });
      await selectShop(selectedShop);
      setCell(null);
      setSearchQuery("");
      setSearchResults([]);
    } catch (e) {
      setError(String(e));
    }
  }

  async function runSearch() {
    if (!searchQuery.trim()) {
      setSearchResults([]);
      return;
    }
    setSearching(true);
    try {
      const results = await invoke<ItemSearchResult[]>("search_items", {
        query: searchQuery.trim(),
      });
      setSearchResults(results);
    } catch (e) {
      setError(String(e));
    } finally {
      setSearching(false);
    }
  }

  async function confirmDelete() {
    if (!deleteConfirm) return;
    try {
      await invoke("delete_shop", { shopVnum: deleteConfirm.vnum });
      setShops((prev) => prev.filter((s) => s.vnum !== deleteConfirm.vnum));
      if (selectedShop?.vnum === deleteConfirm.vnum) {
        setSelectedShop(null);
        setShopItems([]);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setDeleteConfirm(null);
    }
  }

  async function saveMaxValue(next: number) {
    setMaxValue(next);
    try {
      await invoke("set_shop_default_max", {
        shopVnum: mode === "pro-shop" ? (selectedShop?.vnum ?? null) : null,
        value: next,
      });
    } catch (e) {
      setError(String(e));
    }
  }

  async function confirmSync() {
    setSyncing(true);
    setSyncResult(null);
    try {
      const affected = await invoke<number>("sync_shop_stack_sizes", {
        shopVnum: mode === "pro-shop" ? (selectedShop?.vnum ?? null) : null,
        count: maxValue,
      });
      setSyncResult(affected);
      await refreshShops();
      if (selectedShop) await selectShop(selectedShop);
    } catch (e) {
      setError(String(e));
    } finally {
      setSyncing(false);
      setSyncConfirm(false);
    }
  }

  async function saveRename() {
    if (!selectedShop || !nameDraft.trim() || nameDraft === selectedShop.name) {
      setRenaming(false);
      return;
    }
    try {
      await invoke("rename_shop", { shopVnum: selectedShop.vnum, name: nameDraft.trim() });
      const updated = { ...selectedShop, name: nameDraft.trim() };
      setSelectedShop(updated);
      setShops((prev) => prev.map((s) => (s.vnum === updated.vnum ? updated : s)));
    } catch (e) {
      setError(String(e));
    } finally {
      setRenaming(false);
    }
  }

  async function submitNewShop() {
    const npcVnum = Number(newShopNpcVnum);
    if (!newShopName.trim() || !Number.isFinite(npcVnum)) return;
    try {
      await invoke("create_shop", { name: newShopName.trim(), npcVnum });
      await refreshShops();
      setCreating(false);
      setNewShopName("");
      setNewShopNpcVnum("");
    } catch (e) {
      setError(String(e));
    }
  }

  if (checkingConnection) {
    return <p className="text-sm text-muted-foreground">Prüfe Verbindung…</p>;
  }

  if (!connected) {
    return (
      <div className="max-w-md space-y-4">
        <div className="flex items-center gap-2">
          <h1 className="text-2xl font-semibold">{t("nav.shopEditor")}</h1>
          <Button variant="ghost" size="icon-sm" title="Hilfe zu diesem Modul" onClick={() => openManual("shop-editor")}>
            <HelpCircle className="size-4" />
          </Button>
        </div>
        <p className="text-sm text-muted-foreground">
          Verbinde dich mit der MySQL-Datenbank, um Shops zu bearbeiten.
        </p>
        <div className="space-y-2">
          <input
            value={host}
            onChange={(e) => setHost(e.target.value)}
            placeholder="Host"
            className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
          />
          <input
            value={port}
            onChange={(e) => setPort(e.target.value)}
            placeholder="Port"
            className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
          />
          <input
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            placeholder="Benutzername"
            className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
          />
          <PasswordInput value={password} onChange={setPassword} placeholder="Passwort" />
        </div>
        {connError && <p className="text-sm text-destructive">{connError}</p>}
        <Button onClick={handleConnect} disabled={connecting || !host || !username}>
          {connecting ? "Verbinde…" : "Verbinden"}
        </Button>
      </div>
    );
  }

  const filteredShops = shops.filter((s) =>
    s.name.toLowerCase().includes(shopSearch.toLowerCase()),
  );
  const { placedItems, emptyCells, overflowItems, usedSlots } = placeItems(
    shopItems,
    columns,
    rows,
  );

  return (
    <div className="flex h-full gap-4">
      <div className="flex w-72 shrink-0 flex-col gap-2">
        <div className="flex gap-2">
          <div className="relative flex-1">
            <Search className="absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
            <input
              value={shopSearch}
              onChange={(e) => setShopSearch(e.target.value)}
              placeholder="Shops suchen…"
              className="w-full rounded-md border border-border bg-background py-1 pl-7 pr-2 text-sm"
            />
          </div>
          <Button variant="outline" size="icon" onClick={() => setCreating(true)}>
            <Plus className="size-4" />
          </Button>
          <Button variant="ghost" size="icon" title="Hilfe zu diesem Modul" onClick={() => openManual("shop-editor")}>
            <HelpCircle className="size-4" />
          </Button>
        </div>
        <div className="flex-1 space-y-1 overflow-y-auto">
          {filteredShops.map((shop) => (
            <div
              key={shop.vnum}
              onClick={() => selectShop(shop)}
              className={`cursor-pointer rounded-md border border-border p-2 text-sm hover:bg-muted ${
                selectedShop?.vnum === shop.vnum ? "bg-muted" : ""
              }`}
            >
              <div className="font-medium">{shop.name}</div>
              <div className="text-xs text-muted-foreground">
                ID: {shop.vnum} · NPC: {shop.npc_vnum} · {shop.item_count} Gegenstände
              </div>
            </div>
          ))}
        </div>
      </div>

      <div className="min-w-0 flex-1">
        {error && <p className="mb-2 text-sm text-destructive">{error}</p>}
        {!selectedShop && (
          <p className="text-sm text-muted-foreground">Wähle links einen Shop aus.</p>
        )}
        {selectedShop && (
          <div className="flex h-full flex-col gap-3">
            <div className="flex items-start justify-between">
              <div>
                {renaming ? (
                  <input
                    autoFocus
                    value={nameDraft}
                    onChange={(e) => setNameDraft(e.target.value)}
                    onBlur={saveRename}
                    onKeyDown={(e) => e.key === "Enter" && saveRename()}
                    className="rounded-md border border-border bg-background px-2 py-1 text-xl font-semibold"
                  />
                ) : (
                  <h1
                    className="cursor-text text-xl font-semibold"
                    onClick={() => {
                      setNameDraft(selectedShop.name);
                      setRenaming(true);
                    }}
                  >
                    {selectedShop.name}
                  </h1>
                )}
                <p className="text-sm text-muted-foreground">
                  ID: {selectedShop.vnum} · NPC: {selectedShop.npc_vnum} · Gegenstände:{" "}
                  {shopItems.length}
                </p>
              </div>
              <Button variant="destructive" onClick={() => setDeleteConfirm(selectedShop)}>
                <Trash2 className="size-4" />
                Shop löschen
              </Button>
            </div>

            <div className="flex flex-wrap items-center gap-4 rounded-md border border-border p-2 text-sm">
              <div className="flex overflow-hidden rounded-md border border-border">
                <button
                  onClick={() => setMode("global")}
                  className={`px-3 py-1 ${mode === "global" ? "bg-primary text-primary-foreground" : ""}`}
                >
                  Global
                </button>
                <button
                  onClick={() => setMode("pro-shop")}
                  className={`px-3 py-1 ${mode === "pro-shop" ? "bg-primary text-primary-foreground" : ""}`}
                >
                  Pro Shop
                </button>
              </div>
              <Stepper label="Spalten" value={columns} onChange={updateColumns} min={1} max={MAX_SLOTS} />
              <Stepper label="Zeilen" value={rows} onChange={updateRows} min={1} max={MAX_SLOTS} />
              <Stepper label="Max" value={maxValue} onChange={saveMaxValue} min={1} max={9999} step={10} />
              <Button
                variant="outline"
                size="sm"
                onClick={() => {
                  setSyncResult(null);
                  setSyncConfirm(true);
                }}
              >
                <RefreshCw className="size-3.5" />
                Bestandsware synchronisieren
              </Button>
              {syncResult !== null && (
                <span className="text-xs text-muted-foreground">
                  {syncResult} Gegenstände aktualisiert
                </span>
              )}
              <span className="text-xs text-muted-foreground">
                {usedSlots}/{columns * rows} Slots (max. {MAX_SLOTS})
              </span>
              {overflowItems.length > 0 && (
                <span className="flex items-center gap-1 text-xs text-destructive">
                  <AlertTriangle className="size-3.5" />
                  {overflowItems.length} Gegenstand/Gegenstände passen nicht ins Raster
                </span>
              )}
            </div>

            <div className="flex min-h-0 flex-1 gap-3">
              <div
                className="grid content-start gap-1.5 overflow-y-auto"
                style={{
                  gridTemplateColumns: `repeat(${columns}, 48px)`,
                  gridTemplateRows: `repeat(${rows}, 48px)`,
                }}
              >
                {placedItems.map((item) => (
                  <button
                    key={item.item_vnum}
                    title={`${item.item_name} #${item.item_vnum} (1x${item.size})`}
                    onClick={() => setCell({ kind: "filled", index: item.item_vnum, item })}
                    onContextMenu={(e) => {
                      e.preventDefault();
                      removeItem(item);
                    }}
                    style={{
                      gridColumn: item.col,
                      gridRow: `${item.row} / span ${item.span}`,
                    }}
                    className="relative flex w-12 flex-col items-center justify-center rounded-md border border-border bg-muted/40 hover:bg-muted"
                  >
                    {icons[item.item_vnum] ? (
                      <img
                        src={icons[item.item_vnum]!}
                        alt={item.item_name}
                        className="max-h-full w-8 object-contain [image-rendering:pixelated]"
                      />
                    ) : (
                      <span className="text-xs font-medium">{item.item_vnum}</span>
                    )}
                    <span className="absolute bottom-0 right-0.5 text-[9px] leading-none text-muted-foreground">
                      {item.count}
                    </span>
                  </button>
                ))}
                {emptyCells.map(({ row, col }) => (
                  <button
                    key={`empty-${row}-${col}`}
                    onClick={() => setCell({ kind: "empty", index: row * columns + col })}
                    style={{ gridColumn: col, gridRow: row }}
                    className="flex size-12 items-center justify-center rounded-md border border-dashed border-border text-muted-foreground hover:bg-muted/40"
                  >
                    <Plus className="size-4" />
                  </button>
                ))}
              </div>

              <div className="w-64 shrink-0 overflow-hidden rounded-lg border border-border">
                {npcModelLoading && (
                  <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
                    Lädt Modell…
                  </div>
                )}
                {!npcModelLoading && npcModelError && (
                  <div className="flex h-full flex-col items-center justify-center gap-2 p-4 text-center">
                    <AlertTriangle className="size-6 text-destructive" />
                    <p className="text-sm font-medium text-destructive">
                      Modell konnte nicht geladen werden
                    </p>
                    <p className="text-xs text-muted-foreground">{npcModelError}</p>
                  </div>
                )}
                {!npcModelLoading && !npcModelError && (
                  <Gr2Canvas model={npcModel} className="h-full w-full" />
                )}
              </div>
            </div>

            <p className="text-xs text-muted-foreground">
              Leere Zelle klicken zum Hinzufügen · Gegenstand klicken zum Bearbeiten ·
              Rechtsklick zum Entfernen
            </p>
          </div>
        )}
      </div>

      {cell && (
        <Modal onClose={() => setCell(null)}>
          {cell.kind === "filled" ? (
            <div className="space-y-3">
              <div className="flex items-center gap-2">
                {icons[cell.item.item_vnum] && (
                  <img
                    src={icons[cell.item.item_vnum]!}
                    alt={cell.item.item_name}
                    className="size-8 object-contain [image-rendering:pixelated]"
                  />
                )}
                <p className="text-sm font-medium">
                  {cell.item.item_name}{" "}
                  <span className="text-muted-foreground">#{cell.item.item_vnum}</span>
                </p>
              </div>
              <div className="flex items-center gap-2">
                <Button variant="outline" size="icon-sm" onClick={() => changeCount(cell.item, -1)}>
                  <Minus className="size-3.5" />
                </Button>
                <span className="w-10 text-center">{cell.item.count}</span>
                <Button variant="outline" size="icon-sm" onClick={() => changeCount(cell.item, 1)}>
                  <Plus className="size-3.5" />
                </Button>
              </div>
              <Button variant="destructive" onClick={() => removeItem(cell.item)}>
                Entfernen
              </Button>
            </div>
          ) : (
            <div className="space-y-2">
              <div className="flex gap-2">
                <input
                  autoFocus
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && runSearch()}
                  placeholder="Item nach Name oder VNUM suchen…"
                  className="flex-1 rounded-md border border-border bg-background px-2 py-1 text-sm"
                />
                <Button variant="outline" onClick={runSearch} disabled={searching}>
                  <Search className="size-4" />
                </Button>
              </div>
              <div className="max-h-64 space-y-1 overflow-y-auto">
                {searchResults.map((item) => (
                  <div
                    key={item.vnum}
                    className="flex items-center justify-between rounded-md px-2 py-1 text-sm hover:bg-muted"
                  >
                    <span className="flex items-center gap-2">
                      {icons[item.vnum] && (
                        <img
                          src={icons[item.vnum]!}
                          alt={item.name}
                          className="size-6 object-contain [image-rendering:pixelated]"
                        />
                      )}
                      {item.name} <span className="text-muted-foreground">#{item.vnum}</span>
                    </span>
                    <Button size="sm" onClick={() => addItem(item)}>
                      Hinzufügen
                    </Button>
                  </div>
                ))}
              </div>
            </div>
          )}
        </Modal>
      )}

      {creating && (
        <Modal onClose={() => setCreating(false)}>
          <div className="space-y-3">
            <p className="text-sm font-medium">Neuen Shop anlegen</p>
            <input
              value={newShopName}
              onChange={(e) => setNewShopName(e.target.value)}
              placeholder="Shop-Name"
              className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
            <input
              value={newShopNpcVnum}
              onChange={(e) => setNewShopNpcVnum(e.target.value)}
              placeholder="NPC-VNUM"
              className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
            <Button onClick={submitNewShop} disabled={!newShopName.trim() || !newShopNpcVnum}>
              Erstellen
            </Button>
          </div>
        </Modal>
      )}

      {syncConfirm && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="text-sm">
              Alle vorhandenen Gegenstände{" "}
              {mode === "pro-shop" ? (
                <>
                  in <strong>{selectedShop?.name}</strong>
                </>
              ) : (
                <>in allen Shops</>
              )}{" "}
              auf Stapelgröße <strong>{maxValue}</strong> setzen? Das überschreibt die
              aktuellen Mengen und kann nicht rückgängig gemacht werden.
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setSyncConfirm(false)} disabled={syncing}>
                {t("common.cancel")}
              </Button>
              <Button onClick={confirmSync} disabled={syncing}>
                {syncing ? "Synchronisiere…" : "Synchronisieren"}
              </Button>
            </div>
          </div>
        </div>
      )}

      {deleteConfirm && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="w-80 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="text-sm">
              Shop <strong>{deleteConfirm.name}</strong> wirklich löschen? Das kann nicht
              rückgängig gemacht werden.
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setDeleteConfirm(null)}>
                {t("common.cancel")}
              </Button>
              <Button variant="destructive" onClick={confirmDelete}>
                {t("common.delete")}
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function Stepper({
  label,
  value,
  onChange,
  min,
  max,
  step = 1,
}: {
  label: string;
  value: number;
  onChange: (value: number) => void;
  min: number;
  max: number;
  step?: number;
}) {
  return (
    <div className="flex items-center gap-1">
      <span className="text-muted-foreground">{label}:</span>
      <Button
        variant="outline"
        size="icon-sm"
        onClick={() => onChange(Math.max(min, value - step))}
      >
        <Minus className="size-3" />
      </Button>
      <span className="w-8 text-center">{value}</span>
      <Button
        variant="outline"
        size="icon-sm"
        onClick={() => onChange(Math.min(max, value + step))}
      >
        <Plus className="size-3" />
      </Button>
    </div>
  );
}

function Modal({ children, onClose }: { children: React.ReactNode; onClose: () => void }) {
  return (
    <div className="fixed inset-0 flex items-center justify-center bg-black/50">
      <div className="w-96 rounded-lg border border-border bg-card p-4">
        <div className="mb-2 flex justify-end">
          <button onClick={onClose} className="text-muted-foreground hover:text-foreground">
            <X className="size-4" />
          </button>
        </div>
        {children}
      </div>
    </div>
  );
}
