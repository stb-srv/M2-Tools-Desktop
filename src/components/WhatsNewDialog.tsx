import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Sparkles, X } from "lucide-react";
import { readPendingUpdateNotes } from "@/store/updateStore";
import { Button } from "@/components/ui/button";

const LAST_SHOWN_KEY = "m2manager-whats-new-last-shown-version";

/**
 * Shows the real GitHub release notes once, right after the app actually
 * restarts on a newly-installed version - not the same as the small
 * "update available" dot (Sidebar/Settings), which fires *before* installing.
 *
 * There's no built-in "we just updated" event from the Tauri updater plugin,
 * so this is reconstructed from two pieces of local state: whatever notes
 * `updateStore.setAvailable` last persisted (captured the moment any update
 * became known, before the user ever clicks install - see updateStore.ts),
 * and the app's actual running version (`getVersion()`). If they match and
 * we haven't shown this exact version's notes yet, this is the first launch
 * on that version.
 */
export function WhatsNewDialog() {
  const [notes, setNotes] = useState<string | null>(null);
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    (async () => {
      const currentVersion = await getVersion().catch(() => null);
      if (!currentVersion) return;

      const pending = readPendingUpdateNotes();
      if (!pending || pending.version !== currentVersion) return;

      const lastShown = window.localStorage.getItem(LAST_SHOWN_KEY);
      if (lastShown === currentVersion) return;

      setVersion(currentVersion);
      setNotes(pending.notes);
    })();
  }, []);

  function dismiss() {
    if (version) window.localStorage.setItem(LAST_SHOWN_KEY, version);
    setVersion(null);
    setNotes(null);
  }

  if (!version) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="flex max-h-[80vh] w-[32rem] flex-col rounded-lg border border-border bg-card">
        <div className="flex items-center justify-between border-b border-border px-4 py-3">
          <div className="flex items-center gap-2">
            <Sparkles className="size-4 text-primary" />
            <h2 className="text-sm font-semibold">Was ist neu in v{version}</h2>
          </div>
          <Button variant="ghost" size="icon-sm" onClick={dismiss} title="Schließen">
            <X className="size-4" />
          </Button>
        </div>
        <div className="overflow-y-auto p-4">
          {notes ? (
            // No typography plugin in this project (see Quest-Wiki's
            // WikiMarkdown for the same convention) - hand-style each
            // element instead of relying on a `prose` class doing nothing.
            <ReactMarkdown
              remarkPlugins={[remarkGfm]}
              components={{
                h1: (p) => <h1 className="mb-3 text-lg font-semibold" {...p} />,
                h2: (p) => <h2 className="mb-2 mt-4 text-base font-semibold" {...p} />,
                h3: (p) => <h3 className="mb-2 mt-3 text-sm font-medium" {...p} />,
                p: (p) => <p className="mb-3 text-sm leading-relaxed" {...p} />,
                ul: (p) => <ul className="mb-3 list-disc space-y-1 pl-5 text-sm" {...p} />,
                ol: (p) => <ol className="mb-3 list-decimal space-y-1 pl-5 text-sm" {...p} />,
                a: (p) => <a className="text-primary underline" {...p} />,
                code: (p) => <code className="rounded bg-muted px-1 py-0.5 font-mono text-xs" {...p} />,
              }}
            >
              {notes}
            </ReactMarkdown>
          ) : (
            <p className="text-sm text-muted-foreground">Keine Release-Notes für diese Version verfügbar.</p>
          )}
        </div>
        <div className="flex justify-end border-t border-border px-4 py-3">
          <Button onClick={dismiss}>Verstanden</Button>
        </div>
      </div>
    </div>
  );
}
