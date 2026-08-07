import React from "react";
import ReactDOM from "react-dom/client";
import { QuestWiki } from "./features/quest-builder/wiki/QuestWiki";
import "./store/theme";
import "./index.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <QuestWiki />
  </React.StrictMode>,
);
