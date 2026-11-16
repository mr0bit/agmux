/// <reference types="vite/client" />
import React, { lazy, Suspense } from "react";
const LiveApp = lazy(() => import("./LiveApp"));
import { createRoot } from "react-dom/client";
const DemoApp = import.meta.env.DEV ? lazy(() => import("./App")) : null;
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
    ) : DemoApp ? (
      <Suspense fallback={<div>Loading workspace…</div>}>
        <DemoApp />
      </Suspense>
    ) : (
      <div role="alert" style={{ padding: 40 }}>
        Open this application through agmux.exe or agmux gui to connect to your backend.
      </div>
    )}
  </React.StrictMode>,
);
