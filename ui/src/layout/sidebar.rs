//! Navigation links: Favoriten, zuletzt besucht, Gruppen (einklappbar),
//! Einstellungen unten angeheftet. Optional schmale Icon-Leiste.

use crate::components::Icon;
use crate::i18n::t;
use crate::store::navigation::{nav, Category, Section};
use crate::store::update::update_store;
use leptos::prelude::*;

#[component]
fn NavRow(section: Section, rail: bool) -> impl IntoView {
    let n = nav();
    let is_active = move || n.section.get() == section;
    let dirty = move || n.is_dirty(section);
    let update_version = move || {
        if section == Section::Settings {
            update_store().available.get().map(|a| a.version)
        } else {
            None
        }
    };
    let label = move || t(section.label_key());

    if rail {
        return view! {
            <button
                on:click=move |_| n.set_section(section)
                title=label
                class=move || if is_active() {
                    "relative mx-auto flex size-9 items-center justify-center rounded-md transition-colors bg-primary text-primary-foreground"
                } else {
                    "relative mx-auto flex size-9 items-center justify-center rounded-md transition-colors text-foreground hover:bg-muted"
                }
            >
                <Icon name=section.icon() class="size-4 shrink-0"/>
                <Show when=dirty>
                    <span title=move || t("nav.unsavedChanges") class="absolute right-1 top-1 size-1.5 rounded-full bg-warning"></span>
                </Show>
                {move || update_version().map(|v| view! {
                    <span title=format!("Update verfügbar: v{v}") class="absolute right-1 top-1 size-1.5 rounded-full bg-primary"></span>
                })}
            </button>
        }
        .into_any();
    }

    let is_fav = move || n.favorites.with(|f| f.contains(&section));
    view! {
        <div class="group relative flex items-center">
            <button
                on:click=move |_| n.set_section(section)
                class=move || if is_active() {
                    "flex flex-1 items-center gap-2 rounded-md px-3 py-2 text-left text-sm transition-colors bg-primary text-primary-foreground"
                } else {
                    "flex flex-1 items-center gap-2 rounded-md px-3 py-2 text-left text-sm transition-colors text-foreground hover:bg-muted"
                }
            >
                <Icon name=section.icon() class="size-4 shrink-0"/>
                <span class="flex-1 truncate">{label}</span>
                <Show when=dirty>
                    <span
                        title=move || t("nav.unsavedChanges")
                        class=move || if is_active() { "size-1.5 shrink-0 rounded-full bg-primary-foreground" } else { "size-1.5 shrink-0 rounded-full bg-warning" }
                    ></span>
                </Show>
                {move || update_version().map(|v| view! {
                    <span
                        title=format!("Update verfügbar: v{v}")
                        class=move || if is_active() { "size-1.5 shrink-0 rounded-full bg-primary-foreground" } else { "size-1.5 shrink-0 rounded-full bg-primary" }
                    ></span>
                })}
            </button>
            <button
                on:click=move |e| {
                    e.stop_propagation();
                    n.toggle_favorite(section);
                }
                title=move || t(if is_fav() { "nav.unpin" } else { "nav.pin" })
                class=move || {
                    let vis = if is_fav() { "opacity-100" } else { "opacity-0 group-hover:opacity-100" };
                    let col = if is_active() { "text-primary-foreground hover:bg-primary-foreground/20" } else { "text-muted-foreground hover:bg-border" };
                    format!("absolute right-1 rounded p-1 transition-opacity {vis} {col}")
                }
            >
                <Icon name="Star" class=Signal::derive(move || if is_fav() { "size-3 fill-current".to_string() } else { "size-3".to_string() })/>
            </button>
        </div>
    }
    .into_any()
}

fn group_header(label: impl Fn() -> String + Send + Sync + 'static) -> impl IntoView {
    view! { <div class="px-3 pb-1 text-xs font-semibold uppercase tracking-wide text-muted-foreground">{label}</div> }
}

#[component]
pub fn Sidebar() -> impl IntoView {
    let n = nav();
    move || {
        if n.rail_collapsed.get() {
            view! {
                <aside class="flex w-14 flex-col border-r border-border bg-card">
                    <div class="flex items-center justify-center py-4">
                        <button
                            on:click=move |_| n.toggle_rail()
                            title=move || t("nav.expandSidebar")
                            class="rounded-md p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground"
                        >
                            <Icon name="PanelLeft" class="size-4"/>
                        </button>
                    </div>
                    <nav class="flex flex-1 flex-col gap-1 overflow-y-auto px-1 pb-2">
                        {Section::ALL.iter().filter(|s| s.category() != Category::Settings)
                            .map(|s| view! { <NavRow section=*s rail=true/> }).collect_view()}
                    </nav>
                    <div class="border-t border-border px-1 py-2">
                        <NavRow section=Section::Settings rail=true/>
                    </div>
                </aside>
            }
            .into_any()
        } else {
            let recent_visible = move || {
                let favs = n.favorites.get();
                n.recent.get().into_iter().filter(|s| !favs.contains(s)).take(3).collect::<Vec<_>>()
            };
            view! {
                <aside class="flex w-60 flex-col border-r border-border bg-card">
                    <div class="flex items-center justify-between px-4 py-4">
                        <span class="text-lg font-semibold">"M2Manager"</span>
                        <button
                            on:click=move |_| n.toggle_rail()
                            title=move || t("nav.collapseSidebar")
                            class="rounded-md p-1 text-muted-foreground hover:bg-muted hover:text-foreground"
                        >
                            <Icon name="PanelLeftClose" class="size-4"/>
                        </button>
                    </div>
                    <nav class="flex flex-1 flex-col gap-3 overflow-y-auto px-2 pb-2">
                        <Show when=move || n.favorites.with(|f| !f.is_empty())>
                            <div>
                                {group_header(|| t("nav.groups.favorites"))}
                                <div class="flex flex-col gap-0.5">
                                    <For each=move || n.favorites.get() key=|s| *s let:s>
                                        <NavRow section=s rail=false/>
                                    </For>
                                </div>
                            </div>
                        </Show>
                        <Show when=move || !recent_visible().is_empty()>
                            <div>
                                {group_header(|| t("nav.groups.recent"))}
                                <div class="flex flex-col gap-0.5">
                                    <For each=recent_visible key=|s| *s let:s>
                                        <NavRow section=s rail=false/>
                                    </For>
                                </div>
                            </div>
                        </Show>
                        {Category::ORDER.iter().map(|c| {
                            let c = *c;
                            let collapsed = move || n.collapsed.with(|set| set.contains(&c));
                            view! {
                                <div>
                                    <button
                                        on:click=move |_| n.toggle_category(c)
                                        class="flex w-full items-center gap-1 px-3 pb-1 text-xs font-semibold uppercase tracking-wide text-muted-foreground hover:text-foreground"
                                    >
                                        <Icon name=Signal::derive(move || if collapsed() { "ChevronRight" } else { "ChevronDown" }) class="size-3"/>
                                        {move || t(&c.label_key())}
                                    </button>
                                    <Show when=move || !collapsed()>
                                        <div class="flex flex-col gap-0.5">
                                            {Section::ALL.iter().filter(|s| s.category() == c)
                                                .map(|s| view! { <NavRow section=*s rail=false/> }).collect_view()}
                                        </div>
                                    </Show>
                                </div>
                            }
                        }).collect_view()}
                    </nav>
                    <div class="border-t border-border px-2 py-2">
                        <NavRow section=Section::Settings rail=false/>
                    </div>
                </aside>
            }
            .into_any()
        }
    }
}
