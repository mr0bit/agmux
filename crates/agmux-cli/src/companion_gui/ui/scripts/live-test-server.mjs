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
  AGMUX_SOCK: path.join(root, path.basename(root)+"-socket"),
  AGMUX_AUTH_TOKEN_PATH: path.join(root, "token"),
  AGMUX_DB_PATH: path.join(root, "database"),
  AGMUX_CONFIG_PATH: path.join(root, "config"),
  AGMUX_GUI_NO_OPEN: "1",
  GEMINI_CLI_HOME: path.join(root, "gemini-home"),
  AGMUX_GUI_PORT: process.env.AGMUX_TEST_PORT || "5174",
};
if (process.env.AGMUX_GEMINI_TEST === "1") {
  // This suite deliberately verifies missing-auth behavior without account access.
  for (const key of ["GEMINI_API_KEY","GOOGLE_API_KEY","GOOGLE_APPLICATION_CREDENTIALS","GOOGLE_GENAI_USE_VERTEXAI","GOOGLE_CLOUD_PROJECT"]) env[key]="";
}
// Verify the packaged contract with only the two executables, without UI files.
const { copyFile } = await import("node:fs/promises");
const bin = path.join(root, "bin");
await mkdir(bin);
const suffix = process.platform === "win32" ? ".exe" : "";
for (const name of ["agmux", "agmuxd"]) {
  await copyFile(path.join(process.env.AGMUX_TEST_BIN || path.join(repo, "target/debug"), name + suffix), path.join(bin, name + suffix));
}
if (process.env.AGMUX_ACP_FIXTURE === "1") {
  const { chmod } = await import("node:fs/promises");
  const packageRoot=path.join(bin, "node_modules/@google/gemini-cli");
  await mkdir(packageRoot, {recursive:true});
  await copyFile(path.join(ui,"tests/fixtures/acp-provider.mjs"), path.join(packageRoot,"fixture.mjs"));
  await writeFile(path.join(packageRoot,"package.json"), JSON.stringify({bin:{gemini:"fixture.mjs"}}));
  if (process.platform === "win32") {
    await copyFile(process.execPath,path.join(bin,"node.exe"));
    await writeFile(path.join(bin,"gemini.cmd"), "@echo off\r\n");
  } else {
    await writeFile(path.join(bin,"gemini"), `#!${process.execPath}\nimport './node_modules/@google/gemini-cli/fixture.mjs';\n`);
    await chmod(path.join(bin,"gemini"),0o755);
  }
  env.PATH=bin+path.delimiter+(env.PATH || "");
}
delete env.AGMUX_UI_DIST;
const daemon = spawn(path.join(bin, "agmuxd" + suffix), [], {
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
    await access(process.platform === "win32" ? env.AGMUX_AUTH_TOKEN_PATH : env.AGMUX_SOCK);
    break;
  } catch {
    if (daemon.exitCode !== null) throw Error("Daemon exited during startup");
    await new Promise((resolve) => setTimeout(resolve, 20));
  }
}
gui = spawn(path.join(bin, "agmux" + suffix), ["gui"], {
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
