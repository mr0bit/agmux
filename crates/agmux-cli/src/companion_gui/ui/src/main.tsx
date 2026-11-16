import React, { lazy, Suspense } from "react";
const LiveApp = lazy(() => import("./LiveApp"));
import { createRoot } from "react-dom/client";
import App from "./App";
import "./style.css";
import "./design.css";
import "./motion.css";
import "./live.css";

createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    {window.__AGMUX__ ? (
      <Suspense
        fallback={<div style={{ padding: 40 }}>Connecting to Agmux…</div>}
      >
        <LiveApp token={window.__AGMUX__.token} />
      </Suspense>
    ) : (
      <App />
    )}
  </React.StrictMode>,
);
