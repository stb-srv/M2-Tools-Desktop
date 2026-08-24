// Echter E2E-Smoke-Test gegen das gebaute Desktop-Fenster (nicht den
// Vite-Dev-Server + Browser-Stub, den dieses Projekt bisher für "live"-Tests
// genutzt hat - siehe [[m2manager_activity_log]]-Nachfolgeplan, Idee #5).
// Startet `tauri-driver` (WebDriver-Bridge für Tauri-Fenster) gegen
// `msedgedriver.exe`, öffnet die echte gebaute .exe, klickt der Reihe nach
// jeden Sidebar-Bereich über die Command Palette (Strg+K) an und prüft, dass
// eine sichtbare Überschrift statt einer leeren/abgestürzten Seite erscheint.
//
// Voraussetzungen (einmalig, siehe README-Hinweis unten):
//   - `cargo install tauri-driver`
//   - `winget install Microsoft.EdgeDriver` (muss zur installierten Edge-
//     Version passen)
//   - `npm run build` (Frontend) + `cargo build --release` (in src-tauri)
//
// Nutzung: `npm run e2e` (siehe package.json) - baut NICHT automatisch neu,
// das bleibt bewusst ein eigener Schritt (ein Release-Build dauert einige
// Minuten, soll nicht bei jedem Testlauf implizit erneut passieren).

import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { Builder, Key, until } from "selenium-webdriver";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "..");
const exePath = path.join(repoRoot, "src-tauri", "target", "release", "m2manager.exe");

const TAURI_DRIVER_PORT = 4444;

// Reihenfolge/Labels wie in src/store/navigation.ts' NAV_ITEMS + de.json -
// System-Installer bewusst ausgelassen (deaktiviert, nicht in NAV_ITEMS,
// über die Command Palette ohnehin nicht erreichbar).
const SECTION_LABELS = [
  "Dashboard",
  "Server-Steuerung",
  "Quellcode Bauen & Einspielen",
  "Item Editor",
  "Item-Proto-Explorer",
  "Item-Viewer",
  "Modul-Importer",
  "Aufwertungs-Editor",
  "Kisten-Editor",
  "Cube-Editor",
  "Shop-Editor",
  "Mob-Proto-Editor",
  "Mob Drop Editor",
  "Drop-Generator",
  "Quest Builder",
  "Regen-Datei-Editor",
  "Locale-Verwaltung",
  "Server-Events",
  "Account-Verwaltung",
  "GM-Verwaltung",
  "Datenbank-Explorer",
  "Broadcast-System",
  "Tag/Nacht & Schnee",
  "Backup-Browser",
  "Datenbank-Backups",
  "Änderungsprotokoll",
  "TGA Konverter",
  "Icon-Browser",
  "3D-Modell-Viewer",
  "Einstellungen",
];

function findMsedgedriver() {
  const candidate = path.join(
    process.env.LOCALAPPDATA ?? "",
    "Microsoft",
    "WinGet",
    "Packages",
    "Microsoft.EdgeDriver_Microsoft.Winget.Source_8wekyb3d8bbwe",
    "msedgedriver.exe",
  );
  if (existsSync(candidate)) return candidate;
  return "msedgedriver.exe"; // fällt auf PATH zurück, falls anders installiert
}

async function main() {
  if (!existsSync(exePath)) {
    console.error(`Release-Build nicht gefunden: ${exePath}`);
    console.error("Erst `npm run build` und dann `cargo build --release` (in src-tauri) ausführen.");
    process.exit(1);
  }

  const msedgedriver = findMsedgedriver();
  console.log(`tauri-driver wird gestartet (--native-driver ${msedgedriver}, Port ${TAURI_DRIVER_PORT})...`);
  const driverProcess = spawn("tauri-driver", ["--native-driver", msedgedriver, "--port", String(TAURI_DRIVER_PORT)], {
    stdio: ["ignore", "pipe", "pipe"],
  });
  driverProcess.stderr.on("data", (d) => process.stderr.write(`[tauri-driver] ${d}`));

  // tauri-driver braucht einen Moment zum Hochfahren, bevor es Verbindungen annimmt.
  await new Promise((resolve) => setTimeout(resolve, 2000));

  let driver;
  const results = [];
  try {
    driver = await new Builder()
      .usingServer(`http://localhost:${TAURI_DRIVER_PORT}`)
      .withCapabilities({
        "tauri:options": { application: exePath },
        browserName: "wry",
      })
      .build();

    console.log("Fenster gestartet, warte auf die Sidebar...");
    await driver.wait(until.elementLocated({ css: "aside nav" }), 30000);

    for (const label of SECTION_LABELS) {
      const outcome = { label, ok: false, detail: "" };
      try {
        await driver.actions().keyDown(Key.CONTROL).sendKeys("k").keyUp(Key.CONTROL).perform();
        const input = await driver.wait(
          until.elementLocated({ css: "input[placeholder*='Bereich springen']" }),
          5000,
        );
        await input.clear();
        await input.sendKeys(label);
        await driver.sleep(150); // Debounce der gefilterten Ergebnisliste
        await input.sendKeys(Key.ENTER);

        // Nicht auf eine h1/h2 verlassen - Shop-Editor zeigt z.B. vor der
        // Shop-Auswahl bewusst gar keine Seiten-Überschrift (siehe echter
        // Fund unten). Stattdessen: der `<main>`-Inhaltsbereich (App.tsx)
        // muss sichtbaren, über zwei Lesungen im Abstand von 200ms stabilen
        // Text enthalten - robust gegen sowohl leere Übergangszustände als
        // auch Seiten ohne eigene h1/h2. Schwere Chunks (Quest Builder
        // ~476kB, Gr2Canvas ~537kB) brauchen zum Parsen/Mounten spürbar
        // länger als die Command-Palette-Animation, deshalb bis zu 15s
        // pollen statt nur einmal kurz nach dem Enter zu schauen.
        const main = await driver.wait(until.elementLocated({ css: "main" }), 5000);
        const text = await driver.wait(async () => {
          const first = (await main.getText().catch(() => "")).trim();
          if (!first) return null;
          await driver.sleep(200);
          const second = (await main.getText().catch(() => "")).trim();
          return first === second ? first : null;
        }, 15000);
        if (!text) throw new Error("Hauptbereich ist leer");
        outcome.ok = true;
        outcome.detail = text.split("\n")[0].slice(0, 60);
      } catch (e) {
        outcome.detail = String(e).split("\n")[0];
      }
      results.push(outcome);
      console.log(`${outcome.ok ? "✓" : "✗"} ${label} → ${outcome.detail}`);
    }
  } finally {
    if (driver) await driver.quit().catch(() => {});
    driverProcess.kill();
  }

  const failed = results.filter((r) => !r.ok);
  console.log(`\n${results.length - failed.length}/${results.length} Bereiche erfolgreich geladen.`);
  if (failed.length > 0) {
    console.log("Fehlgeschlagen:", failed.map((f) => f.label).join(", "));
    process.exit(1);
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
