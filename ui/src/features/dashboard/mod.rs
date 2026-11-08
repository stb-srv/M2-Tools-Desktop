//! Startseite: Server-Auswahl, Verbindungsstatus, Server-Übersicht,
//! Ressourcen-Verlauf/-Prozesse, Datenbank-Statistik, System-Diagnose und
//! Schnellzugriff.

pub mod resource_history_chart;

use crate::components::{Button, ButtonSize, ButtonVariant, Icon, PageHeader};
use crate::features::connection::ServerSwitcher;
use crate::i18n::t;
use crate::util::format::de_int;
use crate::store::connection_profiles::connection_profiles;
use crate::store::navigation::{nav, Section};
use crate::store::settings_section::{open_settings, SettingsSection};
use crate::tauri::{invoke0, invoke_unit};
use leptos::prelude::*;
use leptos::task::spawn_local;
use resource_history_chart::ResourceHistoryChart;
use serde::Deserialize;
use serde_json::json;

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct HealthCheckResult {
    label: String,
    #[allow(dead_code)]
    kind: String,
    status: String,
    detail: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct DatabaseStats {
    accounts: i64,
    players: i64,
    items: i64,
    shops: i64,
    mobs: i64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct ProcessUsage {
    pid: i64,
    cpu_percent: f64,
    mem_percent: f64,
    rss_kb: f64,
    command: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct MemoryInfo {
    total_bytes: f64,
    free_bytes: f64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct DiskInfo {
    total_kb: f64,
    used_kb: f64,
    #[allow(dead_code)]
    avail_kb: f64,
    capacity_percent: f64,
    mount_point: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct ServerOverview {
    ip_address: Option<String>,
    memory: Option<MemoryInfo>,
    disk: Option<DiskInfo>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Status {
    Checking,
    Ok,
    Error,
}

pub fn format_gib(bytes: f64) -> String {
    format!("{:.1} GB", bytes / 1024f64.powi(3))
}

pub fn format_gib_from_kb(kb: f64) -> String {
    format_gib(kb * 1024.0)
}

/// Wie `String(n)` in JS: ganze Zahlen ohne Nachkommastellen.
fn js_num(n: f64) -> String {
    if n.fract() == 0.0 && n.is_finite() {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

#[component]
pub fn Dashboard() -> impl IntoView {
    let ssh_status = RwSignal::new(Status::Checking);
    let ssh_error = RwSignal::new(None::<String>);
    let mysql_status = RwSignal::new(Status::Checking);
    let mysql_error = RwSignal::new(None::<String>);
    let stats = RwSignal::new(None::<DatabaseStats>);
    let processes = RwSignal::new(None::<Vec<ProcessUsage>>);
    let process_error = RwSignal::new(None::<String>);
    let overview = RwSignal::new(None::<ServerOverview>);
    let overview_error = RwSignal::new(None::<String>);
    let refreshing = RwSignal::new(false);

    let health_results = RwSignal::new(None::<Vec<HealthCheckResult>>);
    let health_running = RwSignal::new(false);
    let health_error = RwSignal::new(None::<String>);

    let run_health_check = move |_| {
        health_running.set(true);
        health_error.set(None);
        spawn_local(async move {
            match invoke0::<Vec<HealthCheckResult>>("run_health_check").await {
                Ok(r) => health_results.set(Some(r)),
                Err(e) => health_error.set(Some(e)),
            }
            health_running.set(false);
        });
    };

    let refresh = move || {
        refreshing.set(true);
        ssh_status.set(Status::Checking);
        mysql_status.set(Status::Checking);
        ssh_error.set(None);
        mysql_error.set(None);
        spawn_local(async move {
            match invoke_unit("test_stored_ssh", &json!({})).await {
                Ok(()) => {
                    ssh_status.set(Status::Ok);
                    match invoke0::<Vec<ProcessUsage>>("get_server_resource_usage").await {
                        Ok(p) => {
                            processes.set(Some(p));
                            process_error.set(None);
                        }
                        Err(e) => {
                            processes.set(None);
                            process_error.set(Some(e));
                        }
                    }
                    match invoke0::<ServerOverview>("get_server_overview").await {
                        Ok(o) => {
                            overview.set(Some(o));
                            overview_error.set(None);
                        }
                        Err(e) => {
                            overview.set(None);
                            overview_error.set(Some(e));
                        }
                    }
                }
                Err(e) => {
                    ssh_status.set(Status::Error);
                    ssh_error.set(Some(e));
                    processes.set(None);
                    overview.set(None);
                }
            }

            let res: Result<(), String> = async {
                let connected = invoke0::<bool>("is_mysql_connected").await?;
                if !connected {
                    mysql_status.set(Status::Error);
                    mysql_error.set(Some("Nicht verbunden".into()));
                    stats.set(None);
                } else {
                    let result = invoke0::<DatabaseStats>("get_database_stats").await?;
                    stats.set(Some(result));
                    mysql_status.set(Status::Ok);
                }
                Ok(())
            }
            .await;
            if let Err(e) = res {
                mysql_status.set(Status::Error);
                mysql_error.set(Some(e));
                stats.set(None);
            }
            refreshing.set(false);
        });
    };

    // Zählt bei jedem Server-Wechsel hoch -> Status/Statistiken neu laden.
    let revision = connection_profiles().revision;
    Effect::new(move |_| {
        revision.track();
        refresh();
        // Alte Diagnose gehört zum vorherigen Server.
        health_results.set(None);
    });

    let ssh_ok = move || ssh_status.get() == Status::Ok;

    view! {
        <div class="space-y-6">
            <div class="flex items-center justify-between">
                <PageHeader module=Section::Dashboard title=Signal::derive(|| t("dashboard.title"))/>
                <Button variant=ButtonVariant::Outline on_click=move |_| refresh() disabled=refreshing>
                    <Icon
                        name="RefreshCw"
                        class=Signal::derive(move || format!("size-4 {}", if refreshing.get() { "animate-spin" } else { "" }))
                    />
                    "Aktualisieren"
                </Button>
            </div>

            <ServerSwitcher/>

            <section class="space-y-2">
                <h2 class="text-sm font-medium text-muted-foreground">{move || t("dashboard.connectionStatus")}</h2>
                <div class="grid gap-3 sm:grid-cols-2">
                    <StatusCard
                        label=Signal::derive(|| t("dashboard.ssh"))
                        status=ssh_status
                        detail=ssh_error
                        on_configure=move |_| open_settings(SettingsSection::Connection)
                    />
                    <StatusCard
                        label=Signal::derive(|| t("dashboard.mysql"))
                        status=mysql_status
                        detail=mysql_error
                        on_configure=move |_| open_settings(SettingsSection::Connection)
                    />
                </div>
            </section>

            <Show when=ssh_ok>
                <section class="space-y-2">
                    <h2 class="text-sm font-medium text-muted-foreground">"Server-Übersicht"</h2>
                    {move || overview_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                    {move || overview.get().map(overview_cards)}
                </section>
            </Show>

            <Show when=ssh_ok>
                <section class="space-y-2">
                    <h2 class="text-sm font-medium text-muted-foreground">{move || t("resourceHistory.title")}</h2>
                    <ResourceHistoryChart/>
                </section>
            </Show>

            <Show when=ssh_ok>
                <section class="space-y-2">
                    <h2 class="text-sm font-medium text-muted-foreground">"Server-Ressourcen"</h2>
                    {move || process_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                    {move || processes.get().map(process_table)}
                </section>
            </Show>

            {move || stats.get().map(|s| view! {
                <section class="space-y-2">
                    <h2 class="text-sm font-medium text-muted-foreground">"Datenbank-Statistik"</h2>
                    <div class="grid grid-cols-2 gap-3 sm:grid-cols-5">
                        <StatCard label="Accounts" value=s.accounts/>
                        <StatCard label="Spieler" value=s.players/>
                        <StatCard label="Items" value=s.items/>
                        <StatCard label="Shops" value=s.shops/>
                        <StatCard label="Mobs/NPCs" value=s.mobs/>
                    </div>
                </section>
            })}

            <section class="space-y-2">
                <div class="flex items-center justify-between">
                    <h2 class="text-sm font-medium text-muted-foreground">"System-Diagnose"</h2>
                    <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=run_health_check disabled=health_running>
                        <Icon
                            name="Stethoscope"
                            class=Signal::derive(move || format!("size-3.5 {}", if health_running.get() { "animate-pulse" } else { "" }))
                        />
                        {move || if health_running.get() { "Prüfe…" } else { "Diagnose starten" }}
                    </Button>
                </div>
                <p class="text-xs text-muted-foreground">
                    "Prüft SSH-/MySQL-Konnektivität und alle konfigurierten lokalen/entfernten Pfade auf einmal, statt dass ein falscher Pfad erst beim Nutzen eines Moduls als stiller Fehlschlag auffällt."
                </p>
                {move || health_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                {move || health_results.get().map(|results| view! {
                    <div class="divide-y divide-border rounded-lg border border-border">
                        {results
                            .into_iter()
                            .map(|r| view! {
                                <div class="flex items-center justify-between gap-3 px-3 py-1.5 text-sm">
                                    <span class="flex min-w-0 items-center gap-2">
                                        {health_status_icon(&r.status)}
                                        <span class="truncate">{r.label}</span>
                                    </span>
                                    <span class="truncate text-xs text-muted-foreground">{r.detail}</span>
                                </div>
                            })
                            .collect_view()}
                    </div>
                })}
            </section>

            <section class="space-y-2">
                <h2 class="text-sm font-medium text-muted-foreground">"Schnellzugriff"</h2>
                <div class="flex flex-wrap gap-2">
                    <QuickAction icon="Terminal" label_key="nav.serverControl" section=Section::ServerControl/>
                    <QuickAction icon="Store" label_key="nav.shopEditor" section=Section::ShopEditor/>
                    <QuickAction icon="Box" label_key="nav.modelViewer" section=Section::ModelViewer/>
                    <QuickAction icon="Settings" label_key="nav.settings" section=Section::Settings/>
                </div>
            </section>
        </div>
    }
}

fn overview_cards(o: ServerOverview) -> impl IntoView {
    let memory = match o.memory {
        Some(m) => {
            let width = (100f64).min((1.0 - m.free_bytes / m.total_bytes) * 100.0);
            view! {
                <div class="mt-1 text-sm">
                    {format!("{} von {} belegt", format_gib(m.total_bytes - m.free_bytes), format_gib(m.total_bytes))}
                </div>
                <div class="mt-1.5 h-1.5 w-full overflow-hidden rounded-full bg-muted">
                    <div class="h-full rounded-full bg-primary" style=format!("width: {}%;", js_num(width))></div>
                </div>
                <p class="mt-1 text-[10px] text-muted-foreground">
                    "Nur echte freie Seiten gezählt, kein wiederverwendbarer Cache - kann von z.B. "
                    <code>"top"</code>
                    " abweichen."
                </p>
            }
            .into_any()
        }
        None => view! { <div class="mt-1 text-sm text-muted-foreground">"Nicht verfügbar"</div> }.into_any(),
    };
    let disk_title = o.disk.as_ref().map(|d| format!("({})", d.mount_point)).unwrap_or_default();
    let disk = match o.disk {
        Some(d) => view! {
            <div class="mt-1 text-sm">
                {format!(
                    "{} von {} belegt ({}%)",
                    format_gib_from_kb(d.used_kb),
                    format_gib_from_kb(d.total_kb),
                    js_num(d.capacity_percent)
                )}
            </div>
            <div class="mt-1.5 h-1.5 w-full overflow-hidden rounded-full bg-muted">
                <div class="h-full rounded-full bg-primary" style=format!("width: {}%;", js_num(d.capacity_percent.min(100.0)))></div>
            </div>
        }
        .into_any(),
        None => view! {
            <div class="mt-1 text-sm text-muted-foreground">
                "Nicht verfügbar - Pfad unter Einstellungen → Server-Pfade prüfen."
            </div>
        }
        .into_any(),
    };
    let ip = o.ip_address.filter(|s| !s.is_empty()).unwrap_or_else(|| "Nicht konfiguriert".into());
    view! {
        <div class="grid gap-3 sm:grid-cols-3">
            <div class="rounded-lg border border-border bg-card p-3">
                <div class="flex items-center gap-2 text-xs text-muted-foreground">
                    <Icon name="Network" class="size-3.5"/>
                    "IP-Adresse"
                </div>
                <div class="mt-1 font-mono text-sm">{ip}</div>
            </div>

            <div class="rounded-lg border border-border bg-card p-3">
                <div class="flex items-center gap-2 text-xs text-muted-foreground">
                    <Icon name="MemoryStick" class="size-3.5"/>
                    "Arbeitsspeicher (RAM)"
                </div>
                {memory}
            </div>

            <div class="rounded-lg border border-border bg-card p-3">
                <div class="flex items-center gap-2 text-xs text-muted-foreground">
                    <Icon name="HardDrive" class="size-3.5"/>
                    "Festplatte "
                    {disk_title}
                </div>
                {disk}
            </div>
        </div>
    }
}

fn process_table(processes: Vec<ProcessUsage>) -> AnyView {
    if processes.is_empty() {
        return view! {
            <p class="text-sm text-muted-foreground">
                "Keine passenden Prozesse gefunden - Prozessnamen unter Einstellungen → Server-Pfade prüfen."
            </p>
        }
        .into_any();
    }
    view! {
        <div class="overflow-hidden rounded-lg border border-border">
            <table class="w-full text-sm">
                <thead class="bg-muted/40 text-xs text-muted-foreground">
                    <tr>
                        <th class="px-3 py-1.5 text-left">"Prozess"</th>
                        <th class="px-3 py-1.5 text-left">"PID"</th>
                        <th class="px-3 py-1.5 text-left">"CPU"</th>
                        <th class="px-3 py-1.5 text-left">"RAM"</th>
                        <th class="px-3 py-1.5 text-left">"RSS"</th>
                    </tr>
                </thead>
                <tbody>
                    {processes
                        .into_iter()
                        .map(|p| view! {
                            <tr class="border-t border-border">
                                <td class="px-3 py-1.5 font-mono">{p.command}</td>
                                <td class="px-3 py-1.5 tabular-nums">{p.pid}</td>
                                <td class="px-3 py-1.5 tabular-nums">{format!("{:.1}%", p.cpu_percent)}</td>
                                <td class="px-3 py-1.5 tabular-nums">{format!("{:.1}%", p.mem_percent)}</td>
                                <td class="px-3 py-1.5 tabular-nums">{format!("{:.0} MB", p.rss_kb / 1024.0)}</td>
                            </tr>
                        })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
    .into_any()
}

#[component]
fn StatusCard(
    #[prop(into)] label: Signal<String>,
    #[prop(into)] status: Signal<Status>,
    #[prop(into)] detail: Signal<Option<String>>,
    #[prop(into)] on_configure: Callback<()>,
) -> impl IntoView {
    view! {
        <div class="rounded-lg border border-border bg-card p-3">
            <div class="flex items-center justify-between">
                <span class="font-medium">{move || label.get()}</span>
                {move || match status.get() {
                    Status::Checking => view! { <span class="text-sm text-muted-foreground">"Prüfe…"</span> }.into_any(),
                    Status::Ok => view! {
                        <span class="flex items-center gap-1 text-sm text-green-600">
                            <Icon name="CheckCircle2" class="size-4"/>
                            "Verbunden"
                        </span>
                    }
                    .into_any(),
                    Status::Error => view! {
                        <span class="flex items-center gap-1 text-sm text-destructive">
                            <Icon name="XCircle" class="size-4"/>
                            "Nicht verbunden"
                        </span>
                    }
                    .into_any(),
                }}
            </div>
            <Show when=move || status.get() == Status::Error>
                <div class="mt-2 space-y-2">
                    {move || detail.get().filter(|d| !d.is_empty()).map(|d| view! {
                        <p class="line-clamp-2 text-xs text-muted-foreground">{d}</p>
                    })}
                    <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| on_configure.run(())>
                        "Einrichten"
                    </Button>
                </div>
            </Show>
        </div>
    }
}

fn health_status_icon(status: &str) -> AnyView {
    match status {
        "ok" => view! { <Icon name="CheckCircle2" class="size-4 shrink-0 text-green-600"/> }.into_any(),
        "warning" => view! { <Icon name="AlertTriangle" class="size-4 shrink-0 text-amber-500"/> }.into_any(),
        "error" => view! { <Icon name="XCircle" class="size-4 shrink-0 text-destructive"/> }.into_any(),
        "skipped" => view! { <Icon name="MinusCircle" class="size-4 shrink-0 text-muted-foreground"/> }.into_any(),
        _ => ().into_any(),
    }
}

#[component]
fn StatCard(label: &'static str, value: i64) -> impl IntoView {
    view! {
        <div class="rounded-lg border border-border bg-card p-3">
            <div class="text-xl font-semibold tabular-nums">{de_int(value)}</div>
            <div class="text-xs text-muted-foreground">{label}</div>
        </div>
    }
}

#[component]
fn QuickAction(icon: &'static str, label_key: &'static str, section: Section) -> impl IntoView {
    view! {
        <Button variant=ButtonVariant::Outline on_click=move |_| nav().set_section(section)>
            <Icon name=icon class="size-4"/>
            {move || t(label_key)}
        </Button>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gib() {
        assert_eq!(format_gib(1024f64.powi(3) * 1.5), "1.5 GB");
        assert_eq!(format_gib_from_kb(1024.0 * 1024.0 * 2.0), "2.0 GB");
        assert_eq!(js_num(42.0), "42");
        assert_eq!(js_num(12.5), "12.5");
    }
}
