import { spawn, execFileSync } from "node:child_process";
import { mkdtemp, mkdir, writeFile, access, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
const ui = path.resolve(fileURLToPath(new URL("..", import.meta.url)));
const repo = path.resolve(ui, "../../../../..");
const root = await mkdtemp(path.join(tmpdir(), "agmux-live-browser-"));
const project = path.join(root, "project");
await mkdir(project);
await writeFile(
  path.join(project, "README.md"),
  "# Live integration workspace\n",
);
execFileSync("git", ["init", "-q", "-b", "main"], { cwd: project });
execFileSync(
  "git",
  [
    "-c",
    "user.name=Agmux Test",
    "-c",
    "user.email=test@local",
    "add",
    "README.md",
  ],
  { cwd: project },
);
execFileSync(
  "git",
  [
    "-c",
    "user.name=Agmux Test",
    "-c",
    "user.email=test@local",
    "commit",
    "-qm",
    "Fixture",
  ],
  { cwd: project },
);
const env = {
  ...process.env,
  AGMUX_SOCK: path.join(root, "socket"),
  AGMUX_AUTH_TOKEN_PATH: path.join(root, "token"),
  AGMUX_DB_PATH: path.join(root, "database"),
  AGMUX_CONFIG_PATH: path.join(root, "config"),
  AGMUX_UI_DIST: path.join(ui, "dist"),
  AGMUX_GUI_PORT: process.env.AGMUX_TEST_PORT || "5174",
};
const daemon = spawn(path.join(repo, "target/debug/agmuxd"), [], {
  cwd: project,
  env,
  stdio: ["ignore", "inherit", "inherit"],
});
let gui;
let stopping = false;
async function stop() {
  if (stopping) return;
  stopping = true;
  gui?.kill("SIGTERM");
  daemon.kill("SIGTERM");
  await new Promise((resolve) => setTimeout(resolve, 300));
  if (gui?.exitCode === null) gui.kill("SIGKILL");
  if (daemon.exitCode === null) daemon.kill("SIGKILL");
  await rm(root, { recursive: true, force: true });
  process.exit(0);
}
process.on("SIGTERM", () => void stop());
process.on("SIGINT", () => void stop());
for (let attempt = 0; attempt < 300; attempt++) {
  try {
    await access(env.AGMUX_SOCK);
    break;
  } catch {
    if (daemon.exitCode !== null) throw Error("Daemon exited during startup");
    await new Promise((resolve) => setTimeout(resolve, 20));
  }
}
gui = spawn(path.join(repo, "target/debug/agmux"), ["gui"], {
  cwd: project,
  env,
  stdio: ["ignore", "inherit", "inherit"],
});
gui.on("exit", (code) => {
  if (!stopping) {
    console.error("Companion exited", code);
    void stop();
  }
});
