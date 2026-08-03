/**
 * Runs an async action (typically an `invoke()` call) with the
 * try/catch/finally boilerplate every loading-state UI needs. `onError` is
 * required, not optional - a `try { ... } finally { ... }` with no `catch`
 * silently swallowed errors twice in this codebase already (Quest Builder's
 * and Mob Drop Editor's item/mob search both did nothing visible on
 * failure, see [[m2manager_quest_builder]]); making the error handler
 * mandatory here means that specific mistake can't happen again wherever
 * this is used instead of a hand-rolled try/catch.
 */
export async function runAsyncAction<T>(
  action: () => Promise<T>,
  handlers: {
    onStart?: () => void;
    onSuccess?: (result: T) => void;
    onError: (message: string) => void;
    onFinally?: () => void;
  },
): Promise<void> {
  handlers.onStart?.();
  try {
    const result = await action();
    handlers.onSuccess?.(result);
  } catch (e) {
    handlers.onError(String(e));
  } finally {
    handlers.onFinally?.();
  }
}
