import { Toast } from "@base-ui/react/toast";
import { CheckCircle2, AlertTriangle, XCircle, Info, X } from "lucide-react";
import { cn } from "@/lib/utils";

export type ToastVariant = "success" | "error" | "warning" | "info";

/**
 * Module-level manager (Base UI's documented pattern for queuing toasts from
 * outside the React tree, e.g. inside an async `runAsyncAction` callback) -
 * see `toast()`/`toast.success()` etc. below for the actual call sites used
 * throughout the app.
 */
export const toastManager = Toast.createToastManager();

interface ToastOptions {
  title?: string;
  timeout?: number;
}

function addToast(variant: ToastVariant, description: string, options?: ToastOptions) {
  return toastManager.add({
    type: variant,
    title: options?.title,
    description,
    timeout: options?.timeout,
  });
}

/**
 * Central entry point for one-shot action feedback (save/delete/create
 * succeeded or failed) - replaces the inline `text-green-600`/`text-destructive`
 * banners that used to be hand-rolled per page. NOT for persistent warnings
 * the user needs to keep seeing while working on a page (e.g. "requires a
 * server restart") - those stay as inline banners, since a toast disappears.
 */
export const toast = Object.assign(
  (description: string, options?: ToastOptions) => addToast("info", description, options),
  {
    success: (description: string, options?: ToastOptions) => addToast("success", description, options),
    error: (description: string, options?: ToastOptions) => addToast("error", description, options),
    warning: (description: string, options?: ToastOptions) => addToast("warning", description, options),
    info: (description: string, options?: ToastOptions) => addToast("info", description, options),
  }
);

const VARIANT_ICON: Record<ToastVariant, typeof CheckCircle2> = {
  success: CheckCircle2,
  error: XCircle,
  warning: AlertTriangle,
  info: Info,
};

const VARIANT_CLASSES: Record<ToastVariant, string> = {
  success: "border-success/30 [&_[data-slot=toast-icon]]:text-success",
  error: "border-destructive/30 [&_[data-slot=toast-icon]]:text-destructive",
  warning: "border-warning/30 [&_[data-slot=toast-icon]]:text-warning",
  info: "border-border [&_[data-slot=toast-icon]]:text-muted-foreground",
};

function ToastList() {
  const { toasts } = Toast.useToastManager();
  return toasts.map((t) => {
    const variant = (t.type as ToastVariant) ?? "info";
    const Icon = VARIANT_ICON[variant];
    return (
      <Toast.Root
        key={t.id}
        toast={t}
        className={cn(
          "pointer-events-auto flex w-full items-start gap-2 rounded-lg border bg-card p-3 text-card-foreground shadow-lg transition-all duration-200",
          "data-[starting-style]:translate-y-2 data-[starting-style]:opacity-0",
          "data-[ending-style]:translate-y-2 data-[ending-style]:opacity-0",
          VARIANT_CLASSES[variant]
        )}
      >
        <Icon data-slot="toast-icon" className="mt-0.5 size-4 shrink-0" />
        <Toast.Content className="min-w-0 flex-1">
          {t.title && <Toast.Title className="text-sm font-medium" />}
          <Toast.Description className="text-sm text-muted-foreground" />
        </Toast.Content>
        <Toast.Close
          aria-label="Schließen"
          className="shrink-0 rounded p-0.5 text-muted-foreground hover:bg-muted hover:text-foreground"
        >
          <X className="size-3.5" />
        </Toast.Close>
      </Toast.Root>
    );
  });
}

/** Mounted once at the app root (see `src/main.tsx`) - not in App.tsx itself,
 * since this is app-wide chrome rather than part of the routed section tree. */
export function Toaster() {
  return (
    <Toast.Provider toastManager={toastManager} timeout={5000}>
      <Toast.Portal>
        <Toast.Viewport className="fixed bottom-4 right-4 z-50 flex w-96 max-w-[calc(100vw-2rem)] flex-col-reverse gap-2 outline-none">
          <ToastList />
        </Toast.Viewport>
      </Toast.Portal>
    </Toast.Provider>
  );
}
