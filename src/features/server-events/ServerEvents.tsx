import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { runAsyncAction } from "@/lib/asyncAction";
import { useNavigationStore } from "@/store/navigation";
import { Button } from "@/components/ui/button";
import {
  Sparkles,
  Info,
  RefreshCw,
  Copy,
  Check,
  RotateCcw,
  ChevronDown,
  ChevronRight,
  Plus,
  Trash2,
  HelpCircle,
} from "lucide-react";
import { openManual } from "@/lib/manual";

interface EventFlagRow {
  name: string;
  value: number;
}

interface FlagDef {
  name: string;
  label: string;
  hint: string;
}

interface SeasonalFlagDef extends FlagDef {
  min: number;
  fallbackDefault: number;
}

// Alle Namen/Beschreibungen/Vorgabewerte direkt aus dem echten
// Server-Quellcode verifiziert (questmanager.cpp, item_manager.cpp,
// char_battle.cpp, cmd_gm.cpp) - siehe EVENT.md für die vollständige
// Herleitung. Nicht geraten, nicht aus generischem Metin2-Wissen ergänzt.
const RATE_FLAGS: FlagDef[] = [
  { name: "mob_item", label: "Item-Drop-Rate", hint: "Multiplikator auf die Drop-Chance jedes Items (z.B. „Doppel-Drop“ = 200)." },
  { name: "mob_exp", label: "EXP-Rate", hint: "Multiplikator auf erhaltene Erfahrungspunkte." },
  { name: "mob_gold", label: "Yang-Betrag", hint: "Multiplikator auf die Höhe eines Yang-Drops (nicht die Chance)." },
  { name: "mob_gold_pct", label: "Yang-Drop-Chance", hint: "Multiplikator auf die Chance, dass überhaupt Yang droppt." },
  { name: "mob_dam", label: "Monster-Schaden", hint: "Multiplikator auf den von Monstern verursachten Schaden." },
  { name: "user_dam", label: "Spieler-Schaden", hint: "Multiplikator auf den von Spielern verursachten Schaden (PvP/PvE)." },
];

const RATE_FLAGS_PREMIUM: FlagDef[] = [
  { name: "mob_item_buyer", label: "Item-Drop-Rate (Premium)", hint: "Wie mob_item, gilt aber nur für Spieler mit aktivem Premium-Item-Timer." },
  { name: "mob_exp_buyer", label: "EXP-Rate (Premium)", hint: "Wie mob_exp, nur für Spieler mit aktivem Premium-EXP-Timer." },
  { name: "mob_gold_buyer", label: "Yang-Betrag (Premium)", hint: "Wie mob_gold, nur für Spieler mit aktivem Premium-Gold-Timer." },
  { name: "mob_gold_pct_buyer", label: "Yang-Drop-Chance (Premium)", hint: "Wie mob_gold_pct, nur für Spieler mit aktivem Premium-Gold-Timer." },
  { name: "user_dam_buyer", label: "Spieler-Schaden (Premium)", hint: "Wie user_dam, nur für Spieler mit aktivem Premium-Timer." },
];

const SEASONAL_FLAGS: SeasonalFlagDef[] = [
  { name: "valentine_drop", label: "Valentinstag-Drop", hint: "Zusätzlicher Sonder-Drop. Item-spezifisch im Quellcode verdrahtet.", min: 1, fallbackDefault: 2000 },
  { name: "halloween_drop", label: "Halloween-Drop", hint: "Zusätzlicher Sonder-Drop.", min: 100, fallbackDefault: 2000 },
  { name: "easter_drop", label: "Oster-Drop", hint: "Zusätzlicher Sonder-Drop.", min: 100, fallbackDefault: 2000 },
  { name: "whiteday_drop", label: "White-Day-Drop", hint: "Zusätzlicher Sonder-Drop.", min: 100, fallbackDefault: 2000 },
  { name: "football_drop", label: "Fußball-Event-Drop", hint: "Zusätzlicher Sonder-Drop.", min: 100, fallbackDefault: 2000 },
  { name: "ramadan_drop", label: "Ramadan-Drop", hint: "Zusätzlicher Sonder-Drop.", min: 100, fallbackDefault: 2000 },
  { name: "icecream_drop", label: "Eis-Drop", hint: "Zusätzlicher Sonder-Drop.", min: 100, fallbackDefault: 2000 },
  { name: "newyear_fire", label: "Neujahr: Feuerwerk-Drop", hint: "Zusätzlicher Sonder-Drop.", min: 100, fallbackDefault: 1000 },
  { name: "newyear_moon", label: "Neujahr: Mond-Drop", hint: "Zusätzlicher Sonder-Drop.", min: 100, fallbackDefault: 500 },
  { name: "new_xmas_event", label: "Weihnachts-Drop", hint: "Zusätzlich nur wirksam, wenn der Spieler ein bestimmtes Item (vnum 53002/53007) besitzt - im Quellcode fest verdrahtet.", min: 50, fallbackDefault: 100 },
  { name: "kids_day_drop", label: "Kindertag-Drop", hint: "Zusätzlicher Sonder-Drop.", min: 100, fallbackDefault: 1000 },
  { name: "kids_day_drop_high", label: "Kindertag-Drop (hochwertig)", hint: "Zusätzlicher Sonder-Drop, seltenere Variante.", min: 100, fallbackDefault: 1000 },
  { name: "medal_part_drop", label: "Medaillen-Teil-Drop", hint: "Nur ab Charakterlevel 30 wirksam - im Quellcode fest verdrahtet.", min: 50, fallbackDefault: 100 },
  { name: "three_skill_item", label: "Drei-Skill-Item-Drop", hint: "Nur ab Level 40 gegen Boss-Monster wirksam - im Quellcode fest verdrahtet.", min: 1, fallbackDefault: 1000 },
  { name: "mars_drop", label: "Mars-Event-Drop", hint: "Zusätzlicher Sonder-Drop, Stärke variiert zusätzlich nach Monster-Rang.", min: 1000, fallbackDefault: 1500 },
  { name: "2006_drop", label: "Jubiläums-Drop (2006)", hint: "Zusätzlicher Sonder-Drop.", min: 100, fallbackDefault: 2000 },
  { name: "2007_drop", label: "Jubiläums-Drop (2007)", hint: "Zusätzlicher Sonder-Drop.", min: 100, fallbackDefault: 2000 },
  { name: "horse_skill_book_drop", label: "Pferde-Skillbuch-Drop", hint: "Zusätzlicher Sonder-Drop.", min: 1000, fallbackDefault: 1000000 },
];

const CURATED_NAMES = new Set([...RATE_FLAGS, ...RATE_FLAGS_PREMIUM, ...SEASONAL_FLAGS].map((f) => f.name));

function gmCommand(name: string, value: string) {
  return `eventflag ${name} ${value || 0}`;
}

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <Button
      variant="ghost"
      size="icon-sm"
      title={`GM-Befehl kopieren: ${text}`}
      onClick={async () => {
        await navigator.clipboard.writeText(text);
        setCopied(true);
        setTimeout(() => setCopied(false), 1500);
      }}
    >
      {copied ? <Check className="size-3.5 text-green-600" /> : <Copy className="size-3.5" />}
    </Button>
  );
}

function FlagRow({
  def,
  currentValue,
  unsetHint,
  onSave,
  onReset,
  busy,
}: {
  def: FlagDef;
  currentValue: number | undefined;
  unsetHint: string;
  onSave: (name: string, value: number) => void;
  onReset: (name: string) => void;
  busy: boolean;
}) {
  const [value, setValue] = useState(currentValue !== undefined ? String(currentValue) : "");

  useEffect(() => {
    setValue(currentValue !== undefined ? String(currentValue) : "");
  }, [currentValue]);

  const isSet = currentValue !== undefined;
  const dirty = value !== (currentValue !== undefined ? String(currentValue) : "");

  return (
    <div className="flex flex-wrap items-center gap-2 border-t border-border py-2 first:border-t-0">
      <div className="min-w-0 flex-1">
        <p className="text-sm font-medium">
          {def.label} <code className="text-xs text-muted-foreground">{def.name}</code>
        </p>
        <p className="text-xs text-muted-foreground">{def.hint}</p>
        {!isSet && <p className="text-xs text-muted-foreground italic">{unsetHint}</p>}
      </div>
      <input
        type="number"
        value={value}
        onChange={(e) => setValue(e.target.value)}
        placeholder="—"
        className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
      />
      <Button size="sm" disabled={!dirty || busy || value === ""} onClick={() => onSave(def.name, Number(value))}>
        Speichern
      </Button>
      {isSet && (
        <Button size="sm" variant="outline" disabled={busy} onClick={() => onReset(def.name)}>
          <RotateCcw className="size-3.5" />
        </Button>
      )}
      <CopyButton text={gmCommand(def.name, value)} />
    </div>
  );
}

function Section({
  title,
  defaultOpen,
  children,
}: {
  title: string;
  defaultOpen: boolean;
  children: React.ReactNode;
}) {
  const [open, setOpen] = useState(defaultOpen);
  return (
    <section className="rounded-lg border border-border p-4">
      <button className="flex w-full items-center gap-2 text-left" onClick={() => setOpen((v) => !v)}>
        {open ? <ChevronDown className="size-4" /> : <ChevronRight className="size-4" />}
        <h2 className="text-sm font-medium text-muted-foreground">{title}</h2>
      </button>
      {open && <div className="mt-2">{children}</div>}
    </section>
  );
}

export function ServerEvents() {
  const setSection = useNavigationStore((s) => s.setSection);
  const [flags, setFlags] = useState<EventFlagRow[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busyName, setBusyName] = useState<string | null>(null);

  const [customName, setCustomName] = useState("");
  const [customValue, setCustomValue] = useState("");

  async function load() {
    await runAsyncAction(() => invoke<EventFlagRow[]>("list_event_flags"), {
      onStart: () => {
        setLoading(true);
        setError(null);
      },
      onSuccess: setFlags,
      onError: setError,
      onFinally: () => setLoading(false),
    });
  }

  useEffect(() => {
    load();
  }, []);

  const flagMap = new Map((flags ?? []).map((f) => [f.name, f.value]));

  async function saveFlag(name: string, value: number) {
    await runAsyncAction(() => invoke("set_event_flag", { name, value }), {
      onStart: () => {
        setBusyName(name);
        setError(null);
      },
      onSuccess: load,
      onError: setError,
      onFinally: () => setBusyName(null),
    });
  }

  async function resetFlag(name: string) {
    await runAsyncAction(() => invoke("delete_event_flag", { name }), {
      onStart: () => {
        setBusyName(name);
        setError(null);
      },
      onSuccess: load,
      onError: setError,
      onFinally: () => setBusyName(null),
    });
  }

  async function addCustom() {
    if (!customName.trim() || customValue.trim() === "") return;
    await saveFlag(customName.trim(), Number(customValue));
    setCustomName("");
    setCustomValue("");
  }

  const customFlags = (flags ?? []).filter((f) => !CURATED_NAMES.has(f.name));

  return (
    <div className="max-w-3xl space-y-6 pb-10">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <h1 className="flex items-center gap-2 text-2xl font-semibold">
            <Sparkles className="size-6" /> Server-Events
          </h1>
          <Button variant="ghost" size="icon-sm" title="Hilfe zu diesem Modul" onClick={() => openManual("server-events")}>
            <HelpCircle className="size-4" />
          </Button>
        </div>
        <Button variant="outline" onClick={load} disabled={loading}>
          <RefreshCw className={`size-4 ${loading ? "animate-spin" : ""}`} />
          Neu laden
        </Button>
      </div>

      <div className="flex items-start gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-400">
        <Info className="mt-0.5 size-4 shrink-0" />
        <span>
          Änderungen hier landen in derselben Tabelle wie der Ingame-GM-Befehl <code>eventflag</code>, wirken
          aber - anders als der Ingame-Befehl - <strong>erst nach einem kompletten Server-Neustart</strong>
          (DB+Game+Login), da die Werte nur einmal beim Start des DB-Prozesses geladen werden.{" "}
          <button className="underline" onClick={() => setSection("server-control")}>
            Zur Server-Steuerung
          </button>
          . Für sofortige Wirkung ohne Neustart: den kopierbaren GM-Befehl neben jedem Wert im Spiel als GM
          eintippen - der wirkt sofort und speichert gleichzeitig in dieselbe Tabelle.
        </span>
      </div>

      {error && <p className="whitespace-pre-wrap text-sm text-destructive">{error}</p>}

      <Section title="Rate-Multiplikatoren (Doppel-Drop, Doppel-EXP, ...)" defaultOpen>
        <p className="mb-2 text-xs text-muted-foreground">
          100 = normal. Server erlaubt 0-1000 (0%-1000%). Ohne gesetzten Wert gilt automatisch 100 (Standard) -
          das ist der Normalzustand, kein "aus".
        </p>
        {RATE_FLAGS.map((def) => (
          <FlagRow
            key={def.name}
            def={def}
            currentValue={flagMap.get(def.name)}
            unsetHint="Nicht gesetzt - aktuell 100% (Standard)."
            onSave={saveFlag}
            onReset={resetFlag}
            busy={busyName === def.name}
          />
        ))}
      </Section>

      <Section title="Premium-Varianten (nur für Spieler mit aktivem Premium-Timer)" defaultOpen={false}>
        {RATE_FLAGS_PREMIUM.map((def) => (
          <FlagRow
            key={def.name}
            def={def}
            currentValue={flagMap.get(def.name)}
            unsetHint="Nicht gesetzt - aktuell 100% (Standard)."
            onSave={saveFlag}
            onReset={resetFlag}
            busy={busyName === def.name}
          />
        ))}
      </Section>

      <Section title="Saison-Events (Sonder-Drops)" defaultOpen>
        <p className="mb-2 text-xs text-muted-foreground">
          Diese Werte sind keine direkte Prozentangabe, sondern eine "je kleiner, desto häufiger"-Stärke
          (Server-Formel: 40000 × Stufen-Faktor ÷ Wert) - Details siehe EVENT.md im Projekt. Ohne gesetzten
          Wert ist das Event komplett <strong>aus</strong> (0), nicht "Standard".
        </p>
        {SEASONAL_FLAGS.map((def) => (
          <FlagRow
            key={def.name}
            def={def}
            currentValue={flagMap.get(def.name)}
            unsetHint={`Nicht gesetzt - Event ist aus. Server-Vorgabewert, falls unterhalb von ${def.min} gesetzt: ${def.fallbackDefault}.`}
            onSave={saveFlag}
            onReset={resetFlag}
            busy={busyName === def.name}
          />
        ))}
      </Section>

      <Section title="Weitere/eigene Flags" defaultOpen>
        <p className="mb-2 text-xs text-muted-foreground">
          Das Flag-System ist beliebig erweiterbar (jeder Name funktioniert) - hier stehen alle in der
          Datenbank gesetzten Flags, die oben in keiner Liste auftauchen.
        </p>
        {customFlags.length === 0 && <p className="text-sm text-muted-foreground">Keine weiteren Flags gesetzt.</p>}
        {customFlags.map((f) => (
          <div key={f.name} className="flex items-center justify-between gap-2 border-t border-border py-2">
            <span className="text-sm">
              <code>{f.name}</code> = {f.value}
            </span>
            <div className="flex items-center gap-1">
              <CopyButton text={gmCommand(f.name, String(f.value))} />
              <Button size="sm" variant="outline" onClick={() => resetFlag(f.name)} disabled={busyName === f.name}>
                <Trash2 className="size-3.5" />
              </Button>
            </div>
          </div>
        ))}
        <div className="mt-3 flex items-end gap-2 border-t border-border pt-3">
          <label className="flex flex-col gap-1 text-xs text-muted-foreground">
            Flag-Name
            <input
              value={customName}
              onChange={(e) => setCustomName(e.target.value)}
              maxLength={32}
              className="w-40 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </label>
          <label className="flex flex-col gap-1 text-xs text-muted-foreground">
            Wert
            <input
              type="number"
              value={customValue}
              onChange={(e) => setCustomValue(e.target.value)}
              className="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
            />
          </label>
          <Button size="sm" onClick={addCustom} disabled={!customName.trim() || customValue.trim() === ""}>
            <Plus className="size-3.5" />
            Hinzufügen
          </Button>
        </div>
      </Section>
    </div>
  );
}
