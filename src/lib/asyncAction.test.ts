import { describe, expect, it, vi } from "vitest";
import { runAsyncAction } from "./asyncAction";

describe("runAsyncAction", () => {
  it("calls onSuccess with the resolved value and onFinally after", async () => {
    const onSuccess = vi.fn();
    const onFinally = vi.fn();
    await runAsyncAction(() => Promise.resolve(42), {
      onSuccess,
      onError: vi.fn(),
      onFinally,
    });
    expect(onSuccess).toHaveBeenCalledWith(42);
    expect(onFinally).toHaveBeenCalled();
  });

  it("always routes a thrown error to onError - the exact bug this exists to prevent", async () => {
    const onError = vi.fn();
    const onSuccess = vi.fn();
    await runAsyncAction(
      () => Promise.reject(new Error("boom")),
      { onSuccess, onError },
    );
    expect(onError).toHaveBeenCalledWith("Error: boom");
    expect(onSuccess).not.toHaveBeenCalled();
  });

  it("runs onFinally even when the action throws", async () => {
    const onFinally = vi.fn();
    await runAsyncAction(() => Promise.reject("nope"), {
      onError: vi.fn(),
      onFinally,
    });
    expect(onFinally).toHaveBeenCalled();
  });

  it("runs onStart before the action settles", async () => {
    const calls: string[] = [];
    await runAsyncAction(
      () => {
        calls.push("action");
        return Promise.resolve();
      },
      {
        onStart: () => calls.push("start"),
        onError: vi.fn(),
      },
    );
    expect(calls).toEqual(["start", "action"]);
  });
});
