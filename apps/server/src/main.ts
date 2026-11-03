// GymOs local server: owns the SQLite file and serves the UI on 127.0.0.1.
import { existsSync } from "node:fs";
import { createServer } from "node:http";
import type { AddressInfo } from "node:net";
import { homedir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { MigrationError, openStore, StoreLockedError, type LocalStore } from "@gymos/store/node";

import { ConfigError, loadConfig, type ServerConfig } from "./config";
import { createDevProxy } from "./dev-proxy";
import { createApp } from "./http";
import { createStaticHandler } from "./static";

const HOST = "127.0.0.1";
/** When this module started running, for the startup time it reports. */
const startedAt = performance.now();

function fail(message: string): never {
  console.error(`\nGymOs couldn't start: ${message}\n`);
  process.exit(1);
}

function readConfig(): ServerConfig {
  try {
    return loadConfig({ platform: process.platform, home: homedir(), cwd: process.cwd(), env: process.env });
  } catch (error) {
    if (error instanceof ConfigError) fail(error.message);
    throw error;
  }
}

function open(config: ServerConfig): LocalStore {
  try {
    return openStore({ dbPath: config.dbPath });
  } catch (error) {
    if (error instanceof StoreLockedError) fail(error.message);
    if (error instanceof MigrationError) {
      const cause = error.cause instanceof Error ? ` (${error.cause.message})` : "";
      fail(`${error.message}${cause}\nDatabase: ${config.dbPath}\nThe file was not changed.`);
    }
    fail(`can't open the database at ${config.dbPath}: ${error instanceof Error ? error.message : String(error)}`);
  }
}

/** The Next.js static export, next to this bundle at apps/server/dist/. */
const webDir = fileURLToPath(new URL("../../web/out/", import.meta.url));

const config = readConfig();
if (!config.devProxy && !existsSync(join(webDir, "index.html"))) {
  fail(`the UI isn't built (${webDir} is missing). Run "pnpm start" from the repo root, which builds it first.`);
}
const store = open(config);
const devProxy = config.devProxy ? createDevProxy(config.devProxy) : null;
const server = createServer(
  createApp({ api: store.api, fallback: devProxy ? devProxy.handle : createStaticHandler(webDir) }),
);
if (devProxy) server.on("upgrade", devProxy.upgrade);

server.on("error", (error: NodeJS.ErrnoException) => {
  store.close();
  if (error.code === "EADDRINUSE") {
    fail(`port ${config.port} is already in use. Stop whatever uses it, or set GYMOS_PORT to another port.`);
  }
  fail(error.message);
});

server.listen(config.port, HOST, () => {
  const { port } = server.address() as AddressInfo;
  const { sqliteVersion, schemaVersion, initMs } = store.info;
  console.log(`
GymOs is running at http://${HOST}:${port}${config.mode === "dev" ? " (dev)" : ""}
Database: ${config.dbPath}
          SQLite ${sqliteVersion}, schema v${schemaVersion}, opened in ${initMs.toFixed(1)} ms
Ready in ${(performance.now() - startedAt).toFixed(0)} ms. Press Ctrl+C to stop.
`);
});

let stopping = false;
function stop(): void {
  if (stopping) return;
  stopping = true;
  server.close();
  server.closeAllConnections();
  store.close();
  console.log("GymOs stopped. Your data is saved.");
  process.exit(0);
}
for (const signal of ["SIGINT", "SIGTERM", "SIGBREAK", "SIGHUP"] as const) process.on(signal, stop);
