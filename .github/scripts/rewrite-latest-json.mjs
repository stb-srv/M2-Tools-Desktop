#!/usr/bin/env node
// Nachbearbeitung nach `tauri-apps/tauri-action`: die Action lädt bereits ein
// `latest.json` als Release-Asset hoch (uploadUpdaterJson, Standard: true),
// dessen `platforms.*.url`-Felder sind aber normale `browser_download_url`s -
// für ein PRIVATES Repo funktionieren die nicht ohne Browser-Session (siehe
// Plan/`immutable-mapping-snowglobe.md`: nur `api.github.com`-Asset-URLs
// lassen sich mit einem reinen API-Token lesen). Dieses Skript ersetzt jede
// URL 1:1 durch die entsprechende `api.github.com/.../releases/assets/{id}`-
// URL (per Dateiname aus der echten Asset-Liste des soeben erstellten
// Release aufgelöst) und schreibt das Ergebnis nach `updater/latest.json` -
// das committet der Workflow-Schritt danach auf `master`, wo die App es über
// eine stabile, über alle Releases hinweg gleichbleibende
// raw.githubusercontent.com-URL abfragt (kein `latest`-Redirect nötig).
import { execSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";

const releaseId = process.argv[2];
if (!releaseId) {
  console.error("Usage: rewrite-latest-json.mjs <releaseId>");
  process.exit(1);
}

const repo = process.env.GITHUB_REPOSITORY;
if (!repo) {
  console.error("GITHUB_REPOSITORY env var not set (expected to run inside GitHub Actions).");
  process.exit(1);
}

function ghApi(path, extraArgs = []) {
  return execSync(["gh", "api", path, ...extraArgs].join(" "), { encoding: "utf8", maxBuffer: 1024 * 1024 * 50 });
}

const assets = JSON.parse(ghApi(`repos/${repo}/releases/${releaseId}/assets`));
const byName = new Map(assets.map((a) => [a.name, a]));

const latestAsset = byName.get("latest.json");
if (!latestAsset) {
  console.error('No "latest.json" asset found on this release - check that uploadUpdaterJson is enabled and that tauri.conf.json has bundle.createUpdaterArtifacts set.');
  process.exit(1);
}

const raw = ghApi(`repos/${repo}/releases/assets/${latestAsset.id}`, ['-H "Accept: application/octet-stream"']);
const manifest = JSON.parse(raw);

for (const [platform, info] of Object.entries(manifest.platforms ?? {})) {
  const filename = decodeURIComponent(new URL(info.url).pathname.split("/").pop());
  const asset = byName.get(filename);
  if (!asset) {
    console.error(`No release asset matches platform "${platform}"'s expected filename "${filename}".`);
    process.exit(1);
  }
  info.url = `https://api.github.com/repos/${repo}/releases/assets/${asset.id}`;
}

mkdirSync("updater", { recursive: true });
writeFileSync("updater/latest.json", JSON.stringify(manifest, null, 2) + "\n");
console.log("Wrote updater/latest.json with authenticated api.github.com asset URLs:");
console.log(JSON.stringify(manifest, null, 2));
