import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { runAsyncAction } from "@/lib/asyncAction";
import { formatRealDropChance } from "@/features/mob-drop-editor/dropChance";
import { Button } from "@/components/ui/button";
import { AlertTriangle, X } from "lucide-react";

interface ShopUsage {
  shop_vnum: number;
  shop_name: string;
  count: number;
}
interface MobDropUsage {
  source: string;
  mob_vnum: number;
  group_name: string;
  percent: number;
}
interface DropGroupUsage {
  mob_vnum: number;
  group_name: string;
  percent: number;
}
interface BoxRewardUsage {
  group_name: string;
  box_vnum: number;
}
interface CubeUsage {
  npc_vnums: number[];
  percent: number;
}
interface EtcDropUsage {
  percent: number;
}
interface QuestSearchLine {
  line_number: number;
  text: string;
}
interface QuestSearchMatch {
  relative_path: string;
  category: string;
  name: string;
  lines: QuestSearchLine[];
}

interface ItemUsageReport {
  shops: ShopUsage[];
  mob_drops: MobDropUsage[];
  etc_drop: EtcDropUsage | null;
  drop_item_groups: DropGroupUsage[];
  is_box_of: string | null;
  possible_reward_in: BoxRewardUsage[];
  cube_ingredient_in: CubeUsage[];
  cube_reward_in: CubeUsage[];
  quests: QuestSearchMatch[];
  warnings: string[];
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="space-y-1">
      <p className="text-xs font-medium text-muted-foreground">{title}</p>
      {children}
    </div>
  );
}

/**
 * "Wo wird das benutzt?" - fasst Shops, Mob-Drops (mob_drop_item.txt/
 * common_drop_item.txt), etc_drop_item.txt, drop_item_group.txt,
 * special_item_group.txt (Kisten), cube.txt und eine Quest-Volltextsuche zu
 * einem Bericht zusammen (`find_item_usages`, siehe
 * src-tauri/src/commands/item_usage.rs). Jede Quelle, die nicht geprüft
 * werden konnte (Pfad nicht konfiguriert, Datei nicht erreichbar), landet
 * unten als Warnung statt den ganzen Bericht scheitern zu lassen.
 */
export function ItemUsageModal({ vnum, onClose }: { vnum: number; onClose: () => void }) {
  const [report, setReport] = useState<ItemUsageReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    runAsyncAction(() => invoke<ItemUsageReport>("find_item_usages", { vnum }), {
      onStart: () => {
        setLoading(true);
        setError(null);
      },
      onSuccess: setReport,
      onError: setError,
      onFinally: () => setLoading(false),
    });
  }, [vnum]);

  const nothingFound =
    report &&
    report.shops.length === 0 &&
    report.mob_drops.length === 0 &&
    !report.etc_drop &&
    report.drop_item_groups.length === 0 &&
    !report.is_box_of &&
    report.possible_reward_in.length === 0 &&
    report.cube_ingredient_in.length === 0 &&
    report.cube_reward_in.length === 0 &&
    report.quests.length === 0;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="flex max-h-[80vh] w-[36rem] flex-col gap-3 rounded-lg border border-border bg-card p-4">
        <div className="flex items-center justify-between">
          <p className="text-sm font-medium">Wo wird Item #{vnum} benutzt?</p>
          <button onClick={onClose} className="text-muted-foreground hover:text-foreground">
            <X className="size-4" />
          </button>
        </div>

        {loading && <p className="text-sm text-muted-foreground">Prüfe…</p>}
        {error && <p className="text-sm text-destructive">{error}</p>}

        {report && (
          <div className="min-h-0 flex-1 space-y-3 overflow-y-auto pr-1 text-sm">
            {nothingFound && (
              <p className="text-sm text-muted-foreground">
                Keine Verwendung in den geprüften Quellen gefunden.
              </p>
            )}

            {report.shops.length > 0 && (
              <Section title={`Shops (${report.shops.length})`}>
                <ul className="space-y-0.5">
                  {report.shops.map((s) => (
                    <li key={s.shop_vnum}>
                      {s.shop_name} (#{s.shop_vnum}) - {s.count}×
                    </li>
                  ))}
                </ul>
              </Section>
            )}

            {report.mob_drops.length > 0 && (
              <Section title={`Mob-Drops (${report.mob_drops.length})`}>
                <ul className="space-y-0.5">
                  {report.mob_drops.map((d, i) => (
                    <li key={i}>
                      Mob #{d.mob_vnum} „{d.group_name}" ({d.source}) - ≈{formatRealDropChance(d.percent)} real
                    </li>
                  ))}
                </ul>
              </Section>
            )}

            {report.etc_drop && (
              <Section title="etc_drop_item.txt">
                <p>≈{formatRealDropChance(report.etc_drop.percent)} real</p>
              </Section>
            )}

            {report.drop_item_groups.length > 0 && (
              <Section title={`drop_item_group.txt (${report.drop_item_groups.length})`}>
                <ul className="space-y-0.5">
                  {report.drop_item_groups.map((d, i) => (
                    <li key={i}>
                      Mob #{d.mob_vnum} „{d.group_name}" - ≈{formatRealDropChance(d.percent)} real
                    </li>
                  ))}
                </ul>
              </Section>
            )}

            {report.is_box_of && (
              <Section title="Kiste (special_item_group.txt)">
                <p>Ist die Kiste der Gruppe „{report.is_box_of}"</p>
              </Section>
            )}

            {report.possible_reward_in.length > 0 && (
              <Section title={`Möglicher Kisten-Inhalt (${report.possible_reward_in.length})`}>
                <ul className="space-y-0.5">
                  {report.possible_reward_in.map((b, i) => (
                    <li key={i}>
                      Gruppe „{b.group_name}" (Kiste #{b.box_vnum})
                    </li>
                  ))}
                </ul>
              </Section>
            )}

            {report.cube_ingredient_in.length > 0 && (
              <Section title={`Cube - als Zutat (${report.cube_ingredient_in.length})`}>
                <ul className="space-y-0.5">
                  {report.cube_ingredient_in.map((c, i) => (
                    <li key={i}>
                      NPC {c.npc_vnums.join(", ")} - {c.percent}% Erfolgschance
                    </li>
                  ))}
                </ul>
              </Section>
            )}

            {report.cube_reward_in.length > 0 && (
              <Section title={`Cube - als Belohnung (${report.cube_reward_in.length})`}>
                <ul className="space-y-0.5">
                  {report.cube_reward_in.map((c, i) => (
                    <li key={i}>
                      NPC {c.npc_vnums.join(", ")} - {c.percent}% Erfolgschance
                    </li>
                  ))}
                </ul>
              </Section>
            )}

            {report.quests.length > 0 && (
              <Section title={`Quests (${report.quests.length}, Volltextsuche nach "${vnum}")`}>
                <ul className="space-y-1">
                  {report.quests.map((q) => (
                    <li key={q.relative_path}>
                      <p className="font-mono text-xs">{q.relative_path}</p>
                      {q.lines.slice(0, 2).map((l) => (
                        <p key={l.line_number} className="truncate pl-2 text-xs text-muted-foreground">
                          Zeile {l.line_number}: {l.text.trim()}
                        </p>
                      ))}
                    </li>
                  ))}
                </ul>
                <p className="text-xs text-muted-foreground">
                  Reine Teilstring-Suche - kann auch Treffer enthalten, bei denen die Zahl in einem
                  anderen Zusammenhang vorkommt (z.B. eine andere vnum, ein Betrag).
                </p>
              </Section>
            )}

            {report.warnings.length > 0 && (
              <p className="flex items-start gap-2 rounded-md border border-amber-500/40 bg-amber-500/10 p-2 text-xs text-amber-700 dark:text-amber-400">
                <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
                <span>{report.warnings.join(" · ")}</span>
              </p>
            )}
          </div>
        )}

        <div className="flex justify-end">
          <Button variant="outline" onClick={onClose}>
            Schließen
          </Button>
        </div>
      </div>
    </div>
  );
}
