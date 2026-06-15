import React from "react";
import ReactDOM from "react-dom/client";
import "@/styles/theme.css";
import "@/lib/i18n"; // 副作用:初始化 i18next
import { App, resolveWindowKind } from "@/App";

const kind = resolveWindowKind();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App kind={kind} />
  </React.StrictMode>,
);
