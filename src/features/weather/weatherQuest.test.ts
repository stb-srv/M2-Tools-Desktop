import { describe, expect, it } from "vitest";
import { gmCommandsFor, generateWeatherQuest, type WeatherState } from "./weatherQuest";

// Same structural smoke test as broadcastQuest.test.ts - every `begin`/`if`
// needs exactly one matching `end` or the server's qc_x64 compiler rejects
// the file outright.
function countKeyword(text: string, keyword: string): number {
  return (text.match(new RegExp(`\\b${keyword}\\b`, "g")) ?? []).length;
}

function expectBalanced(lua: string) {
  const opens = countKeyword(lua, "begin") + countKeyword(lua, "if");
  const ends = countKeyword(lua, "end");
  expect(ends).toBe(opens);
}

describe("generateWeatherQuest", () => {
  it("wraps everything in exactly one quest/state block and stays balanced", () => {
    const lua = generateWeatherQuest({ night_enabled: false, snow_enabled: false, revision: 1 });
    expect(lua).toMatch(/^quest weather_system begin/);
    expect(lua).toContain("state start begin");
    expectBalanced(lua);
  });

  it("inverts night_enabled into the real 'day' flag semantics (1 = dark/night, 0 = light/day)", () => {
    const night = generateWeatherQuest({ night_enabled: true, snow_enabled: false, revision: 5 });
    expect(night).toContain('game.set_event_flag("day", 1)');

    const day = generateWeatherQuest({ night_enabled: false, snow_enabled: false, revision: 5 });
    expect(day).toContain('game.set_event_flag("day", 0)');
  });

  it("maps snow_enabled directly onto xmas_snow", () => {
    const on = generateWeatherQuest({ night_enabled: false, snow_enabled: true, revision: 2 });
    expect(on).toContain('game.set_event_flag("xmas_snow", 1)');

    const off = generateWeatherQuest({ night_enabled: false, snow_enabled: false, revision: 2 });
    expect(off).toContain('game.set_event_flag("xmas_snow", 0)');
  });

  it("gates application behind a revision-armed guard so repeat logins don't re-broadcast", () => {
    const lua = generateWeatherQuest({ night_enabled: true, snow_enabled: true, revision: 4 });
    expect(lua).toContain('game.get_event_flag("weather_rev") ~= 4');
    expect(lua).toContain('game.set_event_flag("weather_rev", 4)');
    expectBalanced(lua);
  });

  const state: WeatherState = { night_enabled: true, snow_enabled: false, revision: 1 };

  it("returns the matching GM commands for instant, no-restart application", () => {
    expect(gmCommandsFor(state)).toEqual(["/eventflag day 1", "/xmas_snow 0"]);
  });
});
