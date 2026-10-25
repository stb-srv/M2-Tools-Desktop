import { SectionTabs } from "@/features/shared/SectionTabs";
import { WeatherControl } from "@/features/weather/WeatherControl";
import { ServerEvents } from "./ServerEvents";

// Tag/Nacht & Schnee setzt ebenfalls nur Event-Flags (`day`, `xmas_snow`) und
// lebt deshalb als zweiter Tab hier statt als eigener Sidebar-Punkt.
export function ServerEventsPage() {
  return (
    <SectionTabs
      tabs={[
        { id: "flags", label: "Event-Flags", content: <ServerEvents /> },
        { id: "weather", label: "Tag/Nacht & Schnee", content: <WeatherControl /> },
      ]}
    />
  );
}
