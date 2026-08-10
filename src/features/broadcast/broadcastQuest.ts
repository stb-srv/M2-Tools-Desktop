// Generates the single "Broadcast/Broadcast_System.lua" quest file from the
// locally stored message list. Verified directly against the real server
// source (game-src/source/game/src), not guessed from generic Metin2 docs:
// - notice_all() (questlua_global.cpp:847) is a genuine global broadcast.
// - server_loop_timer(name, seconds) (questlua_global.cpp:154) is a real,
//   player-independent repeating server timer - fires `when <name>.server_timer`.
// - game.get_event_flag/set_event_flag (questlua_game.cpp:16/26) are global
//   in-memory flags, used here to arm each message's loop timer exactly once
//   per server run (on the first `login` after a reload/restart).
//
// Each message's timer/flag name includes its revision number (bumped on
// every edit, see broadcast.rs::update_message) - CQuestManager::Reload()
// clears its name->timer-id map but does NOT cancel already-pending C++
// timer events, so a stale event from before a reload would otherwise be
// able to resolve against a wrongly-reused id. Changing the name on every
// edit means a stale old-revision event finds nothing after a reload
// instead of misfiring - see the plan's "wichtige Falle" note.

export interface BroadcastMessage {
  id: number;
  text: string;
  interval_minutes: number;
  enabled: boolean;
  revision: number;
  created_at: string;
}

// Same escaping as Quest Builder's luaString() (questTemplates.ts) - notice
// banners are a single line, so newlines are collapsed to spaces instead of
// the dialog-only "[ENTER]" marker.
function luaString(text: string): string {
  const escaped = text
    .replace(/\\/g, "\\\\")
    .replace(/"/g, '\\"')
    .replace(/\r\n|\n/g, " ");
  return `"${escaped}"`;
}

function timerName(message: BroadcastMessage): string {
  return `bc_${message.id}_${message.revision}`;
}

function flagName(message: BroadcastMessage): string {
  return `bc_armed_${message.id}_${message.revision}`;
}

export function generateBroadcastQuest(messages: BroadcastMessage[]): string {
  const active = messages.filter((m) => m.enabled);

  const lines: string[] = ["quest broadcast_system begin", "\tstate start begin"];

  if (active.length > 0) {
    lines.push("\t\twhen login begin");
    for (const message of active) {
      const flag = flagName(message);
      const timer = timerName(message);
      const seconds = Math.round(message.interval_minutes * 60);
      lines.push(`\t\t\tif game.get_event_flag("${flag}") == 0 then`);
      lines.push(`\t\t\t\tgame.set_event_flag("${flag}", 1)`);
      lines.push(`\t\t\t\tserver_loop_timer("${timer}", ${seconds})`);
      lines.push("\t\t\tend");
    }
    lines.push("\t\tend");

    for (const message of active) {
      lines.push("");
      lines.push(`\t\twhen ${timerName(message)}.server_timer begin`);
      lines.push(`\t\t\tnotice_all(${luaString(message.text)})`);
      lines.push("\t\tend");
    }
  }

  lines.push("\tend");
  lines.push("end");

  return lines.join("\n") + "\n";
}
