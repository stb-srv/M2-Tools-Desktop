import { describe, expect, it } from "vitest";
import { isBackupDue, nextDueAt } from "./schedule";

describe("isBackupDue", () => {
  const now = Date.parse("2026-08-25T12:00:00.000Z");

  it("is never due when disabled", () => {
    expect(isBackupDue(now, null, 0)).toBe(false);
    expect(isBackupDue(now, "2020-01-01T00:00:00.000Z", 0)).toBe(false);
  });

  it("is due immediately if no automatic backup has ever run", () => {
    expect(isBackupDue(now, null, 24)).toBe(true);
  });

  it("is due once the interval has fully elapsed", () => {
    const exactly24hAgo = new Date(now - 24 * 60 * 60 * 1000).toISOString();
    expect(isBackupDue(now, exactly24hAgo, 24)).toBe(true);
  });

  it("is not due before the interval elapses", () => {
    const oneHourAgo = new Date(now - 60 * 60 * 1000).toISOString();
    expect(isBackupDue(now, oneHourAgo, 24)).toBe(false);
  });

  it("treats an unparseable timestamp as due", () => {
    expect(isBackupDue(now, "not-a-date", 24)).toBe(true);
  });
});

describe("nextDueAt", () => {
  it("returns null when disabled", () => {
    expect(nextDueAt(null, 0)).toBeNull();
  });

  it("returns now when no automatic backup has ever run", () => {
    expect(nextDueAt(null, 24)).toBeInstanceOf(Date);
  });

  it("adds the interval to the last run timestamp", () => {
    const last = "2026-08-25T00:00:00.000Z";
    const due = nextDueAt(last, 12);
    expect(due?.toISOString()).toBe("2026-08-25T12:00:00.000Z");
  });
});
