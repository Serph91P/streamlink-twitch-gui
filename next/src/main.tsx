import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "./App";
import { BrowserBackend, TauriBackend } from "./api/backend";
import "./styles.css";

const root = document.getElementById("root");

if (!root) {
  throw new Error("Missing application root");
}

createRoot(root).render(
  <StrictMode>
    <App
      backend={
        "__TAURI_INTERNALS__" in window
          ? new TauriBackend()
          : new BrowserBackend()
      }
    />
  </StrictMode>,
);
