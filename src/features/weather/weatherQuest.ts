// Generates the single "Weather/Weather_System.lua" quest file from the
// locally stored toggle state. Verified directly against the real server
// source (game-src/source/game/src), not guessed from generic Metin2 docs:
// - The "day" event flag has semantics opposite to its name:
//   `questmanager.cpp:1172-1189` shows value 1 -> "DayMode dark" (night
//   forced), value 0 -> "DayMode light" (day). This module only ever exposes
//   "night_enabled" to the rest of the app and flips the flag value here.
// - "xmas_snow" (`xmas_event.cpp:13-27`) is the snow particle effect,
//   1 = on, 0 = off - straightforward, no inversion.
// - game.get_event_flag/set_event_flag (questlua_game.cpp:16/26) run the
//   real `CQuestManager::SetEventFlag`, which pushes to every connected
//   client immediately and persists to the DB - unlike a raw SQL write to
//   the same table (see db/event_flags.rs), which only takes effect after a
//   full server restart. Newly connecting players are re-synced
//   automatically by the server's own `BroadcastEventFlagOnLogin`
//   (questmanager.cpp:1374), so this quest only needs to apply the flags
//   once, not on every login.
// - "weather_rev" gates that one-time application per revision, same
//   mechanism as the Broadcast-System's per-message armed flag
//   (broadcastQuest.ts) - without it, every single login on the server would
//   re-run both set_event_flag calls and re-broadcast to everyone online.

export interface WeatherState {
  night_enabled: boolean;
  snow_enabled: boolean;
  revision: number;
}

export function generateWeatherQuest(state: WeatherState): string {
  const dayFlagValue = state.night_enabled ? 1 : 0;
  const snowFlagValue = state.snow_enabled ? 1 : 0;

  const lines = [
    "quest weather_system begin",
    "\tstate start begin",
    "\t\twhen login begin",
    `\t\t\tif game.get_event_flag("weather_rev") ~= ${state.revision} then`,
    `\t\t\t\tgame.set_event_flag("day", ${dayFlagValue})`,
    `\t\t\t\tgame.set_event_flag("xmas_snow", ${snowFlagValue})`,
    `\t\t\t\tgame.set_event_flag("weather_rev", ${state.revision})`,
    "\t\t\tend",
    "\t\tend",
    "\tend",
    "end",
  ];

  return lines.join("\n") + "\n";
}

// Copyable fallback for instant effect without waiting for the next login -
// same GM commands the generated quest calls under the hood
// (`cmd.cpp:189,286`), both GM_HIGH_WIZARD only.
export function gmCommandsFor(state: Pick<WeatherState, "night_enabled" | "snow_enabled">): string[] {
  return [
    `/eventflag day ${state.night_enabled ? 1 : 0}`,
    `/xmas_snow ${state.snow_enabled ? 1 : 0}`,
  ];
}
