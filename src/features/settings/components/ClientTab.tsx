import { PathSetting, SectionHeader, SettingsCard } from "./shared";

export function ClientTab() {
  return (
    <div className="space-y-4">
      <SectionHeader
        title="Client & Werkzeuge"
        description="Lokale Ordner und Programme auf diesem PC."
      />

      <SettingsCard title="Spiel-Client">
        <PathSetting
          def={{
            key: "client_path",
            label: "Client-Ordner",
            directory: true,
            dialogTitle: "Metin2-Client-Ordner auswählen",
            hint: (
              <>
                Ordner des Metin2-Clients (enthält <code>granny2.dll</code>) - für 3D-Modelle,
                Item-Icons, Icon-Browser und Modul-Importer.
              </>
            ),
          }}
        />
        <PathSetting
          def={{
            key: "npclist_path",
            label: "NPC-Liste (npclist.txt)",
            dialogTitle: "npclist.txt auswählen",
            filters: [{ name: "Textdatei", extensions: ["txt"] }],
            emptyText: "Automatisch suchen",
            clearable: true,
            hint: (
              <>
                Normalerweise nicht nötig: M2Manager sucht zuerst unter{" "}
                <code>root\npclist.txt</code> und sonst im ganzen Client-Ordner. Nur setzen, wenn
                die Datei umbenannt wurde oder außerhalb des Clients liegt.
              </>
            ),
          }}
        />
      </SettingsCard>

      <SettingsCard title="Externe Werkzeuge" description="Werden vom Item Editor beim Anlegen und Bearbeiten von Items aufgerufen.">
        <PathSetting
          def={{
            key: "eterpack_tool_path",
            label: "EterPackConsoleLz4.exe",
            dialogTitle: "EterPackConsoleLz4.exe auswählen",
            filters: [{ name: "Programm", extensions: ["exe"] }],
            hint: (
              <>
                Packt <code>icon.epk</code> neu, nachdem ein Item-Icon hinzugefügt wurde. Liegt
                normalerweise unter <code>Client\pack\EterPackConsoleLz4.exe</code>.
              </>
            ),
          }}
        />
        <PathSetting
          def={{
            key: "mysql2proto_dir",
            label: "Mysql2Proto-Ordner",
            directory: true,
            dialogTitle: "Mysql2Proto-Ordner auswählen",
            hint: (
              <>
                Ordner mit <code>Mysql2Proto.exe</code> und <code>Mysql2Proto.json</code> - erzeugt
                aus der Datenbank eine neue <code>item_proto</code> für den Client.
              </>
            ),
          }}
        />
      </SettingsCard>
    </div>
  );
}
