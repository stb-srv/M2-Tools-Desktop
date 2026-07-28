import { useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Minus, Plus, Search, Trash2, X } from "lucide-react";

interface ShopSummary {
  vnum: number;
  name: string;
  npc_vnum: number;
  npc_name: string;
  item_count: number;
}

interface ShopItem {
  item_vnum: number;
  item_name: string;
  count: number;
}

interface ItemSearchResult {
  vnum: number;
  name: string;
}

export function ShopEditor() {
  const { t } = useTranslation();

  const [connected, setConnected] = useState(false);
  const [connecting, setConnecting] = useState(false);
  const [connError, setConnError] = useState<string | null>(null);
  const [host, setHost] = useState("");
  const [port, setPort] = useState("3306");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");

  const [shops, setShops] = useState<ShopSummary[]>([]);
  const [selectedShop, setSelectedShop] = useState<ShopSummary | null>(null);
  const [shopItems, setShopItems] = useState<ShopItem[]>([]);
  const [loadingShops, setLoadingShops] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<ItemSearchResult[]>([]);
  const [searching, setSearching] = useState(false);

  const [deleteConfirm, setDeleteConfirm] = useState<ShopSummary | null>(null);

  async function handleConnect() {
    setConnecting(true);
    setConnError(null);
    try {
      await invoke("connect_mysql", {
        config: { host, port: Number(port), username, database: null },
        password,
      });
      setConnected(true);
      await refreshShops();
    } catch (e) {
      setConnError(String(e));
    } finally {
      setConnecting(false);
    }
  }

  async function refreshShops() {
    setLoadingShops(true);
    setError(null);
    try {
      const result = await invoke<ShopSummary[]>("list_shops");
      setShops(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoadingShops(false);
    }
  }

  async function selectShop(shop: ShopSummary) {
    setSelectedShop(shop);
    setError(null);
    try {
      const items = await invoke<ShopItem[]>("get_shop_items", {
        shopVnum: shop.vnum,
      });
      setShopItems(items);
    } catch (e) {
      setError(String(e));
    }
  }

  async function changeCount(item: ShopItem, delta: number) {
    if (!selectedShop) return;
    const nextCount = Math.max(1, item.count + delta);
    setShopItems((prev) =>
      prev.map((i) =>
        i.item_vnum === item.item_vnum ? { ...i, count: nextCount } : i,
      ),
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
        count: 1,
      });
      await selectShop(selectedShop);
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

  if (!connected) {
    return (
      <div className="max-w-md space-y-4">
        <h1 className="text-2xl font-semibold">{t("nav.shopEditor")}</h1>
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
          <input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            placeholder="Passwort"
            className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
          />
        </div>
        {connError && <p className="text-sm text-destructive">{connError}</p>}
        <Button onClick={handleConnect} disabled={connecting || !host || !username}>
          {connecting ? "Verbinde…" : "Verbinden"}
        </Button>
      </div>
    );
  }

  return (
    <div className="flex h-full gap-4">
      <div className="w-72 shrink-0 space-y-2 overflow-y-auto">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-medium text-muted-foreground">
            Shops ({shops.length})
          </h2>
          <Button variant="ghost" size="sm" onClick={refreshShops}>
            {loadingShops ? "…" : "↻"}
          </Button>
        </div>
        {shops.map((shop) => (
          <div
            key={shop.vnum}
            onClick={() => selectShop(shop)}
            className={`cursor-pointer rounded-md border border-border p-2 text-sm hover:bg-muted ${
              selectedShop?.vnum === shop.vnum ? "bg-muted" : ""
            }`}
          >
            <div className="flex items-center justify-between">
              <span className="font-medium">{shop.name}</span>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  setDeleteConfirm(shop);
                }}
                className="text-muted-foreground hover:text-destructive"
              >
                <Trash2 className="size-3.5" />
              </button>
            </div>
            <div className="text-xs text-muted-foreground">
              NPC: {shop.npc_name} · {shop.item_count} Items
            </div>
          </div>
        ))}
      </div>

      <div className="min-w-0 flex-1 space-y-4">
        {error && <p className="text-sm text-destructive">{error}</p>}
        {!selectedShop && (
          <p className="text-sm text-muted-foreground">
            Wähle links einen Shop aus.
          </p>
        )}
        {selectedShop && (
          <>
            <h1 className="text-xl font-semibold">{selectedShop.name}</h1>

            <div className="space-y-1">
              {shopItems.map((item) => (
                <div
                  key={item.item_vnum}
                  className="flex items-center justify-between rounded-md border border-border px-3 py-1.5 text-sm"
                >
                  <span>
                    {item.item_name}{" "}
                    <span className="text-muted-foreground">
                      #{item.item_vnum}
                    </span>
                  </span>
                  <div className="flex items-center gap-2">
                    <Button
                      variant="outline"
                      size="icon-sm"
                      onClick={() => changeCount(item, -1)}
                    >
                      <Minus className="size-3.5" />
                    </Button>
                    <span className="w-8 text-center">{item.count}</span>
                    <Button
                      variant="outline"
                      size="icon-sm"
                      onClick={() => changeCount(item, 1)}
                    >
                      <Plus className="size-3.5" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon-sm"
                      onClick={() => removeItem(item)}
                    >
                      <X className="size-3.5" />
                    </Button>
                  </div>
                </div>
              ))}
            </div>

            <div className="space-y-2 rounded-lg border border-border p-3">
              <div className="flex gap-2">
                <input
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
              {searchResults.length > 0 && (
                <div className="max-h-48 space-y-1 overflow-y-auto">
                  {searchResults.map((item) => (
                    <div
                      key={item.vnum}
                      className="flex items-center justify-between rounded-md px-2 py-1 text-sm hover:bg-muted"
                    >
                      <span>
                        {item.name}{" "}
                        <span className="text-muted-foreground">
                          #{item.vnum}
                        </span>
                      </span>
                      <Button size="sm" onClick={() => addItem(item)}>
                        Hinzufügen
                      </Button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </>
        )}
      </div>

      {deleteConfirm && (
        <div className="fixed inset-0 flex items-center justify-center bg-black/50">
          <div className="w-80 space-y-3 rounded-lg border border-border bg-card p-4">
            <p className="text-sm">
              Shop <strong>{deleteConfirm.name}</strong> wirklich löschen? Das
              kann nicht rückgängig gemacht werden.
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
