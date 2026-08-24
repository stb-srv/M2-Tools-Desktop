import React from "react";
import ReactDOM from "react-dom/client";
// Muss vor allem anderen importiert werden (reiner Seiteneffekt beim
// Modul-Laden setzt/entfernt die `dark`-Klasse auf <html>) - vorher wurde
// dieses Modul nur von lazy geladenen Features importiert (Quest Builder,
// Settings/GeneralTab.tsx), lief also beim App-Start gar nicht, bis eines
// davon zum ersten Mal geladen wurde. Echter Nutzer-Bugreport: App startete
// deshalb immer im Hellmodus, Dunkelmodus griff erst nach einem Besuch bei
// „Einstellungen" (dem einzigen Ort, der useThemeStore importiert und damit
// dieses Modul erstmals lädt).
import "@/store/theme";
import App from "./App";
import "./index.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
