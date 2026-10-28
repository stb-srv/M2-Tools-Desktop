import { HelpCircle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { openManual } from "@/lib/manual";
import { EMBEDDED_MODULES, NAV_ITEMS, hostSection, type ModuleId } from "@/store/navigation";
import { cn } from "@/lib/utils";

/**
 * Einheitlicher Seitenkopf: Symbol des Bereichs (wie in der Sidebar), Titel
 * und Hilfe-Knopf, rechts optional eigene Aktionen. Module, die als Tab in
 * einem anderen Bereich stecken, bekommen eine kleinere Variante, damit nicht
 * zwei gleich große Überschriften übereinander stehen.
 */
export function PageHeader({
  module,
  title,
  actions,
  className,
}: {
  module: ModuleId;
  title: React.ReactNode;
  actions?: React.ReactNode;
  className?: string;
}) {
  const embedded = EMBEDDED_MODULES.some((e) => e.module === module);
  const Icon = NAV_ITEMS.find((item) => item.section === hostSection(module))?.icon;

  return (
    <div className={cn("flex flex-wrap items-center justify-between gap-3", className)}>
      <div className="flex min-w-0 items-center gap-3">
        {Icon && !embedded && (
          <div className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-primary text-primary-foreground shadow-sm">
            <Icon className="size-5" />
          </div>
        )}
        <h1 className={cn("min-w-0 font-semibold", embedded ? "text-lg" : "text-2xl")}>{title}</h1>
        <Button
          variant="ghost"
          size="icon-sm"
          title="Hilfe zu diesem Modul"
          onClick={() => openManual(module)}
        >
          <HelpCircle className="size-4" />
        </Button>
      </div>
      {actions && <div className="flex flex-wrap items-center gap-2">{actions}</div>}
    </div>
  );
}
