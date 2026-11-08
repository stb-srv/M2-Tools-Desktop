use super::super::duplicates::{DuplicateItemFinding, DuplicateMobFinding};
use super::super::types::{MobDropGroup, NumberingIssue};
use super::super::{with_group, IconMap};
use super::shared::{fmt_num, Modal};
use crate::components::{Button, ButtonSize, ButtonVariant, Icon};
use leptos::prelude::*;

/// Zeigt alle Prüfungen (Duplikate, siehe `duplicates.rs`, plus die
/// Nummerierungs-Prüfung aus mobdrop::check_numbering) und bietet je Fund
/// passende Aktionen: "Anpassen" springt zum betroffenen Mob im Haupteditor,
/// "Löschen" entfernt genau diese eine Dopplung.
#[component]
pub fn DuplicatesModal(
    #[prop(into)] duplicate_items: Signal<Vec<DuplicateItemFinding>>,
    #[prop(into)] duplicate_mobs: Signal<Vec<DuplicateMobFinding>>,
    #[prop(into)] numbering_issues: Signal<Vec<NumberingIssue>>,
    groups: RwSignal<Option<Vec<MobDropGroup>>>,
    icons: RwSignal<IconMap>,
    #[prop(into)] on_navigate_to_group: Callback<usize>,
    #[prop(into)] on_delete_item_occurrence: Callback<(usize, usize)>,
    #[prop(into)] on_delete_group: Callback<usize>,
    #[prop(into)] on_close: Callback<()>,
) -> impl IntoView {
    let opt_str = |s: Option<String>| s.unwrap_or_default();
    view! {
        <Modal on_close=on_close width_class="w-[36rem]">
            <div class="max-h-[70vh] space-y-4 overflow-y-auto">
                <p class="text-sm font-medium">"Prüfung"</p>

                <div class="space-y-2">
                    <p class="text-xs font-medium text-muted-foreground">
                        {move || format!("Fehlerhafte Nummerierung ({})", numbering_issues.with(|n| n.len()))}
                    </p>
                    <p class="text-xs text-muted-foreground">
                        "Der Server liest jeden Drop-Eintrag über seine laufende Nummer (1, 2, 3, …) - fehlt eine Zahl, bricht das Einlesen an der Lücke ab und alle folgenden Einträge dieses Mobs gehen auf dem Server verloren, auch wenn hier alles vollständig angezeigt wird. Speichern behebt dies automatisch (die Nummerierung wird immer neu vergeben)."
                    </p>
                    <Show when=move || numbering_issues.with(|n| n.is_empty())>
                        <p class="text-sm text-muted-foreground">"Nummerierung überall korrekt."</p>
                    </Show>
                    {move || numbering_issues.get().into_iter().map(|issue| {
                        let gi = issue.group_index;
                        let found = issue.found.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(", ");
                        view! {
                            <div class="flex items-center justify-between gap-2 rounded-md border border-border p-2">
                                <span class="flex items-start gap-2 text-sm">
                                    <Icon name="AlertTriangle" class="mt-0.5 size-3.5 shrink-0 text-amber-500"/>
                                    <span>
                                        {move || opt_str(with_group(groups, gi, |g| g.name.clone()))}
                                        " "
                                        <span class="text-muted-foreground">
                                            {move || format!("(Mob #{})", opt_str(with_group(groups, gi, |g| fmt_num(g.mob_vnum))))}
                                        </span>
                                        {format!(" - gefunden: {found}")}
                                    </span>
                                </span>
                                <Button size=ButtonSize::Sm variant=ButtonVariant::Outline on_click=move |_| on_navigate_to_group.run(gi)>
                                    <Icon name="Pencil" class="size-3.5"/>
                                    "Anpassen"
                                </Button>
                            </div>
                        }
                    }).collect_view()}
                </div>

                <div class="space-y-2">
                    <p class="text-xs font-medium text-muted-foreground">
                        {move || format!("Doppelte Items pro Mob ({})", duplicate_items.with(|d| d.len()))}
                    </p>
                    <Show when=move || duplicate_items.with(|d| d.is_empty())>
                        <p class="text-sm text-muted-foreground">"Keine doppelten Items in einem Mob gefunden."</p>
                    </Show>
                    {move || duplicate_items.get().into_iter().map(|finding| {
                        let gi = finding.group_index;
                        let item_vnum = finding.item_vnum;
                        view! {
                            <div class="space-y-1 rounded-md border border-border p-2">
                                <div class="flex items-center justify-between">
                                    <span class="text-sm">
                                        {finding.mob_name.clone()}
                                        " "
                                        <span class="text-muted-foreground">{format!("(Mob #{})", fmt_num(finding.mob_vnum))}</span>
                                        {format!(" - Item #{} {}× eingetragen", fmt_num(item_vnum), finding.item_indices.len())}
                                    </span>
                                    <Button size=ButtonSize::Sm variant=ButtonVariant::Outline on_click=move |_| on_navigate_to_group.run(gi)>
                                        <Icon name="Pencil" class="size-3.5"/>
                                        "Anpassen"
                                    </Button>
                                </div>
                                <div class="flex flex-wrap gap-1">
                                    {finding.item_indices.iter().enumerate().map(|(occurrence, &ii)| {
                                        let percent = move || groups.with(|g| {
                                            g.as_ref().and_then(|g| g.get(gi)).and_then(|g| g.items.get(ii)).map(|i| fmt_num(i.percent))
                                        });
                                        view! {
                                            <button
                                                on:click=move |_| on_delete_item_occurrence.run((gi, ii))
                                                class="flex items-center gap-1 rounded-md border border-border bg-muted/40 px-2 py-1 text-xs hover:bg-destructive/10 hover:text-destructive"
                                            >
                                                {move || icons.with(|m| m.get(&(item_vnum as i64)).cloned().flatten()).map(|src| view! {
                                                    <img src=src alt="" class="size-4 object-contain [image-rendering:pixelated]"/>
                                                })}
                                                {move || format!("Eintrag {} ({}%)", occurrence + 1, percent().unwrap_or_default())}
                                                <Icon name="X" class="size-3"/>
                                            </button>
                                        }
                                    }).collect_view()}
                                </div>
                            </div>
                        }
                    }).collect_view()}
                </div>

                <div class="space-y-2">
                    <p class="text-xs font-medium text-muted-foreground">
                        {move || format!("Doppelte Mob-Einträge ({})", duplicate_mobs.with(|d| d.len()))}
                    </p>
                    <Show when=move || duplicate_mobs.with(|d| d.is_empty())>
                        <p class="text-sm text-muted-foreground">"Keine mehrfach vorkommenden Mobs gefunden."</p>
                    </Show>
                    {move || duplicate_mobs.get().into_iter().map(|finding| view! {
                        <div class="space-y-1 rounded-md border border-border p-2">
                            <p class="text-sm">
                                {format!(
                                    "Mob #{} kommt {}× als eigener Eintrag vor",
                                    fmt_num(finding.mob_vnum),
                                    finding.group_indices.len(),
                                )}
                            </p>
                            <div class="space-y-1">
                                {finding.group_indices.iter().map(|&gi| view! {
                                    <div class="flex items-center justify-between rounded-md border border-border bg-muted/40 px-2 py-1 text-xs">
                                        <span>
                                            {move || {
                                                let (name, t, n) = with_group(groups, gi, |g| (g.name.clone(), g.drop_type.clone(), g.items.len()))
                                                    .unwrap_or_default();
                                                format!("{name} · {t} · {n} Drops")
                                            }}
                                        </span>
                                        <div class="flex items-center gap-1">
                                            <Button size=ButtonSize::Sm variant=ButtonVariant::Outline on_click=move |_| on_navigate_to_group.run(gi)>
                                                <Icon name="Pencil" class="size-3.5"/>
                                                "Anpassen"
                                            </Button>
                                            <Button size=ButtonSize::Sm variant=ButtonVariant::Destructive on_click=move |_| on_delete_group.run(gi)>
                                                <Icon name="X" class="size-3.5"/>
                                                "Löschen"
                                            </Button>
                                        </div>
                                    </div>
                                }).collect_view()}
                            </div>
                        </div>
                    }).collect_view()}
                </div>
            </div>
        </Modal>
    }
}
