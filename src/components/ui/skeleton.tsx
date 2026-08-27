import type { ComponentProps } from "react";
import { cn } from "@/lib/utils";

/** Pulsing placeholder block for list/table loading states - replaces the
 * bare "Lade…" text that was scattered per-page before this. */
function Skeleton({ className, ...props }: ComponentProps<"div">) {
  return <div data-slot="skeleton" className={cn("animate-pulse rounded-md bg-muted", className)} {...props} />;
}

export { Skeleton };
