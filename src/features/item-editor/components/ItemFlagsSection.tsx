import { useState } from "react";
import { AlertTriangle } from "lucide-react";
import {
  ITEM_FLAGS,
  WEAR_FLAGS,
  ANTI_FLAGS,
  IMMUNE_FLAGS,
  LIMIT_TYPES,
  APPLY_TYPES,
  VALUE_HINTS,
  VALUE_LABELS_BY_TYPE,
  weaponDisplayDamage,
  weaponAttackSpeedLabel,
  weaponEffectiveAttackSpeed,
  WEAPON_SUBTYPE_TWO_HANDED,
} from "../itemFlags";
import type { ItemProtoInput } from "../types";
import { Field } from "./shared";

function FlagGroup({
  title,
  options,
  value,
  onToggle,
  onRaw,
}: {
  title: string;
  options: { value: number; label: string }[];
  value: number;
  onToggle: (bit: number) => void;
  onRaw: (value: number) => void;
}) {
  return (
    <div className="space-y-1">
      <div className="flex items-center gap-2">
        <p className="text-xs font-medium text-muted-foreground">{title}</p>
        <input
          type="number"
          value={value}
          onChange={(e) => onRaw(Number(e.target.value) || 0)}
          className="w-24 rounded-md border border-border bg-background px-1.5 py-0.5 text-xs"
        />
      </div>
      <div className="flex flex-wrap gap-2">
        {options.map((opt) => (
          <label key={opt.value} className="flex items-center gap-1 text-xs">
            <input
              type="checkbox"
              checked={(value & opt.value) !== 0}
              onChange={() => onToggle(opt.value)}
            />
            {opt.label}
          </label>
        ))}
      </div>
    </div>
  );
}

export function ItemFlagsSection({
  item,
  set,
  toggleFlag,
}: {
  item: ItemProtoInput;
  set: <K extends keyof ItemProtoInput>(key: K, value: ItemProtoInput[K]) => void;
  toggleFlag: (key: "wearflag" | "antiflag" | "immuneflag" | "flag", bit: number) => void;
}) {
  const [advancedOpen, setAdvancedOpen] = useState(false);

  function num(value: string) {
    const n = Number(value);
    return Number.isFinite(n) ? n : 0;
  }

  return (
    <section className="space-y-3 rounded-lg border border-border p-4">
      <button
        className="text-sm font-medium text-muted-foreground"
        onClick={() => setAdvancedOpen((v) => !v)}
      >
        Flags & Werte (erweitert) {advancedOpen ? "▲" : "▼"}
      </button>
      {advancedOpen && (
        <div className="space-y-4">
          <p className="text-xs text-muted-foreground">
            Bits aus dem echten Server-Quellcode verifiziert (length.h/item_length.h). Rohwerte
            (Hex/Dezimal) bleiben trotzdem immer verfügbar.
          </p>

          <FlagGroup
            title={`Trageort (wearflag = ${item.wearflag})`}
            options={WEAR_FLAGS}
            value={item.wearflag}
            onToggle={(bit) => toggleFlag("wearflag", bit)}
            onRaw={(v) => set("wearflag", v)}
          />
          <FlagGroup
            title={`Klassen-/Geschlechtssperre (antiflag = ${item.antiflag})`}
            options={ANTI_FLAGS}
            value={item.antiflag}
            onToggle={(bit) => toggleFlag("antiflag", bit)}
            onRaw={(v) => set("antiflag", v)}
          />
          <FlagGroup
            title={`Immunität (immuneflag = ${item.immuneflag})`}
            options={IMMUNE_FLAGS}
            value={item.immuneflag}
            onToggle={(bit) => toggleFlag("immuneflag", bit)}
            onRaw={(v) => set("immuneflag", v)}
          />

          <FlagGroup
            title={`Item-Eigenschaften (flag = ${item.flag})`}
            options={ITEM_FLAGS}
            value={item.flag}
            onToggle={(bit) => toggleFlag("flag", bit)}
            onRaw={(v) => set("flag", v)}
          />

          <div className="flex flex-wrap gap-3">
            <Field label="Limit-Typ 0">
              <select
                value={item.limittype0}
                onChange={(e) => set("limittype0", num(e.target.value))}
                className="rounded-md border border-border bg-background px-2 py-1 text-sm"
              >
                <option value={0}>—</option>
                {LIMIT_TYPES.map((t) => (
                  <option key={t.value} value={t.value}>
                    {t.label} ({t.value})
                  </option>
                ))}
              </select>
            </Field>
            <Field label="Wert">
              <input
                type="number"
                value={item.limitvalue0}
                onChange={(e) => set("limitvalue0", num(e.target.value))}
                className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
              />
            </Field>
          </div>

          <p className="text-xs text-muted-foreground">
            {VALUE_HINTS[item.type] ??
              "value0-5 (Effektwerte) hängen vom Typ/Subtyp ab und wurden für diesen Typ noch nicht im Quellcode nachgeschlagen."}{" "}
            Am zuverlässigsten über "Referenz-Item übernehmen" oben befüllen und dann anpassen.
          </p>
          <div className="grid grid-cols-3 gap-2 sm:grid-cols-6">
            {(["value0", "value1", "value2", "value3", "value4", "value5"] as const).map(
              (k, i) => (
                <Field key={k} label={VALUE_LABELS_BY_TYPE[item.type]?.[i] ?? k}>
                  <input
                    type="number"
                    value={item[k]}
                    onChange={(e) => set(k, num(e.target.value))}
                    className="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                  />
                </Field>
              ),
            )}
          </div>

          {item.type === 1 && (
            <div className="rounded-md border border-border bg-muted/40 p-2 text-xs">
              <p className="font-medium">Anzeige im Client (Tooltip-Vorschau):</p>
              {(() => {
                const { min, max } = weaponDisplayDamage(item.value3, item.value4, item.value5);
                const effectiveSpeed = weaponEffectiveAttackSpeed(item.value0, item.subtype);
                return (
                  <p>
                    Angriffskraft: {max > min ? `${min} ~ ${max}` : `${min}`} · Angriffstempo:{" "}
                    {weaponAttackSpeedLabel(effectiveSpeed)}
                    {item.subtype === WEAPON_SUBTYPE_TWO_HANDED && (
                      <> ({item.value0} - 10 Zweihand-Malus = {effectiveSpeed})</>
                    )}
                    {(item.value1 > 0 || item.value2 > 0) && (
                      <> · Magie-Angriff: {item.value1} ~ {item.value2}</>
                    )}
                  </p>
                );
              })()}
            </div>
          )}
          {item.type === 2 && (
            <div className="rounded-md border border-border bg-muted/40 p-2 text-xs">
              <p className="font-medium">Anzeige im Client (Tooltip-Vorschau):</p>
              <p>
                Verteidigung: {item.value1 + item.value5 * 2}
                {item.value0 > 0 && <> · Magie-Verteidigung: {item.value0}</>}
              </p>
            </div>
          )}
          {item.type === 3 && item.subtype === 2 && (
            <div className="space-y-1 rounded-md border border-border bg-muted/40 p-2 text-xs">
              <p className="font-medium">Aufwertungs-Schriftrolle (USE_TUNING):</p>
              <p>
                value0 = Schriftrollen-ID — 0-6 sind fest im Server-Quellcode belegt (Chukbok,
                Hyuniron, Yongsin, Musin, Yagong, Memo, B-Dragon). Für eine neue, generische
                Schriftrolle einen Wert ≥ 7 verwenden.
              </p>
              {item.value0 >= 0 && item.value0 <= 6 && (
                <p className="flex items-center gap-1 text-amber-600 dark:text-amber-400">
                  <AlertTriangle className="size-3.5 shrink-0" /> value0 = {item.value0} kollidiert mit
                  einer fest verdrahteten Alt-Schriftrolle — für eine neue generische Schriftrolle
                  mindestens 7 verwenden.
                </p>
              )}
              <p>
                value2 = Erfolgschance in % (0-100), value3 = Verhalten bei Fehlschlag (0 = Item wird
                abgestuft falls möglich, 1 = Item bleibt bei Fehlschlag erhalten). Erfordert den
                char_item.cpp-Patch für generische Boost-Schriftrollen (bereits auf dem Server
                eingespielt, wirkt erst nach dem nächsten Bauen &amp; Einspielen der Server-Software).
              </p>
            </div>
          )}

          <p className="text-xs text-muted-foreground">
            Stat-Boni (applytype/applyvalue) — Typen aus dem echten Server-Quellcode verifiziert
            (<code>enum EApplyTypes</code>).
          </p>
          <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
            {(
              [
                ["applytype0", "applyvalue0"],
                ["applytype1", "applyvalue1"],
                ["applytype2", "applyvalue2"],
                ["applytype3", "applyvalue3"],
              ] as const
            ).map(([tKey, vKey]) => (
              <div key={tKey} className="flex items-end gap-2">
                <Field label="Typ">
                  <select
                    value={item[tKey]}
                    onChange={(e) => set(tKey, num(e.target.value))}
                    className="w-56 rounded-md border border-border bg-background px-2 py-1 text-sm"
                  >
                    {APPLY_TYPES.map((t) => (
                      <option key={t.value} value={t.value}>
                        {t.label} ({t.value})
                      </option>
                    ))}
                  </select>
                </Field>
                <Field label="Wert">
                  <input
                    type="number"
                    value={item[vKey]}
                    onChange={(e) => set(vKey, num(e.target.value))}
                    className="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                  />
                </Field>
              </div>
            ))}
          </div>
        </div>
      )}
    </section>
  );
}
