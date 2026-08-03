// Verified against this server's own source:
//   item_manager_read_tables.cpp:665-667 (ReadMonsterDropItemGroup, "drop"
//     branch) - the file's percent value is stored as dwPct = percent × 10000.
//   item_manager.cpp:664-698 (GetDropPct) - iDeltaPercent starts at 100
//     (baseline, no level delta/luck bonus - aiPercentByDeltaLev[15] = 100
//     at equal killer/mob level, constants.cpp:242) and iRandRange starts
//     at 4,000,000.
//   item_manager.cpp:867-871 (CreateDropItem, drop-group branch) -
//     iPercent = (dwPct × iDeltaPercent) / 100, rolled against
//     number(1, iRandRange).
//
// Under baseline conditions this collapses to
//   iPercent / iRandRange = (percent × 10000 × 100 / 100) / 4,000,000 = percent / 400
// - a FRACTION (0..1), not a percentage. An earlier version of this app
// displayed that fraction directly with a "%" suffix, off by a factor of
// 100 (confirmed against real in-game testing: value 10 felt far more
// common than the "0.03%" that used to show). The correct percentage is
// that fraction × 100, i.e. percent / 4.
export function realDropChancePercent(percent: number): number {
  return percent / 4;
}

export function formatRealDropChance(percent: number): string {
  const fixed = realDropChancePercent(percent)
    .toFixed(2)
    .replace(/0+$/, "")
    .replace(/\.$/, "");
  return `${fixed || "0"}%`;
}
