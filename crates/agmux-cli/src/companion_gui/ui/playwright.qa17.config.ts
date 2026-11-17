import base from "./playwright.live.config";
import { defineConfig } from "@playwright/test";
export default defineConfig({...base, testMatch:"qa17.spec.ts", webServer:{...base.webServer, command:"node scripts/live-test-server.mjs", env:{AGMUX_ACP_FIXTURE:"1"}}, timeout:60000});
