import { describe, expect, it } from "vitest";
import { generateBroadcastQuest, type BroadcastMessage } from "./broadcastQuest";

// Same structural smoke test as questTemplates.test.ts - every `begin`/`if`
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

function message(patch: Partial<BroadcastMessage>): BroadcastMessage {
  return {
    id: 1,
    text: "Willkommen auf dem Server!",
    interval_minutes: 30,
    enabled: true,
    revision: 1,
    created_at: "2026-08-09T00:00:00Z",
    ...patch,
  };
}

describe("generateBroadcastQuest", () => {
  it("wraps everything in exactly one quest/state block and stays balanced", () => {
    const lua = generateBroadcastQuest([message({})]);
    expect(lua).toMatch(/^quest broadcast_system begin/);
    expect(lua).toContain("state start begin");
    expectBalanced(lua);
  });

  it("emits a login-armed guard and a server_timer handler using the interval in seconds", () => {
    const lua = generateBroadcastQuest([message({ id: 7, revision: 3, interval_minutes: 15 })]);
    expect(lua).toContain('game.get_event_flag("bc_armed_7_3")');
    expect(lua).toContain('game.set_event_flag("bc_armed_7_3", 1)');
    expect(lua).toContain('server_loop_timer("bc_7_3", 900)');
    expect(lua).toContain("when bc_7_3.server_timer begin");
    expect(lua).toContain('notice_all("Willkommen auf dem Server!")');
  });

  it("leaves out disabled messages entirely", () => {
    const lua = generateBroadcastQuest([message({ id: 1, enabled: false })]);
    expect(lua).not.toContain("bc_1_1");
    expect(lua).not.toContain("when login begin");
    expectBalanced(lua);
  });

  it("escapes quotes and backslashes and collapses newlines", () => {
    const lua = generateBroadcastQuest([
      message({ text: 'Sag "Hallo"\\Willkommen!\nZeile 2' }),
    ]);
    expect(lua).toContain('notice_all("Sag \\"Hallo\\"\\\\Willkommen! Zeile 2")');
  });

  it("handles several messages with independent timers in one login block", () => {
    const lua = generateBroadcastQuest([
      message({ id: 1, revision: 1, interval_minutes: 10 }),
      message({ id: 2, revision: 1, interval_minutes: 20, text: "Zweite Nachricht" }),
    ]);
    expect(lua).toContain('server_loop_timer("bc_1_1", 600)');
    expect(lua).toContain('server_loop_timer("bc_2_1", 1200)');
    expect(lua).toContain("when bc_1_1.server_timer begin");
    expect(lua).toContain("when bc_2_1.server_timer begin");
    expectBalanced(lua);
  });
});
