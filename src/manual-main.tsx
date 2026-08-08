import React from "react";
import ReactDOM from "react-dom/client";
import { Manual } from "./features/manual/Manual";
import "./store/theme";
import "./index.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Manual />
  </React.StrictMode>,
);
