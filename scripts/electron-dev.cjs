const { spawn } = require("node:child_process");
const fs = require("node:fs");
const net = require("node:net");
const { resolve } = require("node:path");

const electron = require("electron");
const appRoot = resolve(__dirname, "..");
const viteBin = resolve(appRoot, "node_modules", ".bin", process.platform === "win32" ? "vite.cmd" : "vite");
const env = { ...process.env };
const vitePort = Number(process.env.VITE_PORT || 5173);
const snapDesktopKeys = [
  "CHROME_DESKTOP",
  "GDK_BACKEND",
  "GDK_PIXBUF_MODULEDIR",
  "GDK_PIXBUF_MODULE_FILE",
  "GIO_LAUNCHED_DESKTOP_FILE",
  "GIO_LAUNCHED_DESKTOP_FILE_PID",
  "GIO_MODULE_DIR",
  "GSETTINGS_SCHEMA_DIR",
  "GTK_EXE_PREFIX",
  "GTK_IM_MODULE_FILE",
  "GTK_MODULES",
  "GTK_PATH",
  "LOCPATH",
  "VSCODE_NLS_CONFIG",
  "XDG_DATA_HOME",
];

delete env.ELECTRON_RUN_AS_NODE;

if (env.XDG_DATA_DIRS_VSCODE_SNAP_ORIG) {
  env.XDG_DATA_DIRS = env.XDG_DATA_DIRS_VSCODE_SNAP_ORIG;
}

for (const key of snapDesktopKeys) {
  delete env[key];
}

for (const key of Object.keys(env)) {
  const value = env[key];
  if (
    key === "SNAP" ||
    key.startsWith("SNAP_") ||
    (typeof value === "string" && (value.includes("/snap/code") || value.includes("/snap/code/")))
  ) {
    delete env[key];
  }
}

function newestMutterXauthority(runtimeDir) {
  if (!runtimeDir) {
    return null;
  }

  try {
    return fs
      .readdirSync(runtimeDir)
      .filter((name) => name.startsWith(".mutter-Xwaylandauth."))
      .map((name) => {
        const path = resolve(runtimeDir, name);
        return { path, mtimeMs: fs.statSync(path).mtimeMs };
      })
      .sort((left, right) => right.mtimeMs - left.mtimeMs)[0]?.path ?? null;
  } catch {
    return null;
  }
}

const xauthority = newestMutterXauthority(env.XDG_RUNTIME_DIR);
if (xauthority) {
  env.XAUTHORITY = xauthority;
}

function waitForPort(port, host = "127.0.0.1", timeoutMs = 30000) {
  const startedAt = Date.now();

  return new Promise((resolveReady, rejectReady) => {
    function tryConnect() {
      const socket = net.createConnection({ host, port });

      socket.once("connect", () => {
        socket.end();
        resolveReady();
      });

      socket.once("error", () => {
        socket.destroy();
        if (Date.now() - startedAt >= timeoutMs) {
          rejectReady(new Error(`Timed out waiting for ${host}:${port}`));
          return;
        }
        setTimeout(tryConnect, 100);
      });
    }

    tryConnect();
  });
}

function stop(child) {
  if (child && !child.killed) {
    child.kill("SIGTERM");
  }
}

async function main() {
  let vite = null;

  try {
    await waitForPort(vitePort, "127.0.0.1", 250);
  } catch {
    vite = spawn(process.execPath, [viteBin, "--host", "127.0.0.1", "--port", String(vitePort)], {
      cwd: appRoot,
      env,
      stdio: "inherit",
    });

    await waitForPort(vitePort);
  }

  const args = [];

  if (process.platform === "linux") {
    args.push("--no-sandbox", "--disable-vulkan", "--in-process-gpu", "--ozone-platform=x11");
  }

  args.push(appRoot);

  const electronProcess = spawn(electron, args, {
    cwd: appRoot,
    env,
    stdio: "inherit",
  });

  const cleanup = () => {
    stop(electronProcess);
    stop(vite);
  };

  process.once("SIGINT", cleanup);
  process.once("SIGTERM", cleanup);

  electronProcess.on("exit", (code, signal) => {
    stop(vite);
    if (signal) {
      console.error(`${electron} exited with signal ${signal}`);
      process.exit(1);
    }
    process.exit(code ?? 1);
  });
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
});
