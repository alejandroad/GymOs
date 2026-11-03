import { posix, win32 } from "node:path";

export const DEFAULT_PORT = 4321;
/** `pnpm dev` runs beside `pnpm start` without a port clash. */
export const DEFAULT_DEV_PORT = 4322;
export const DB_FILE_NAME = "gymos.sqlite3";

export class ConfigError extends Error {
  override name = "ConfigError";
}

export interface ServerConfig {
  /** `dev` proxies the UI to `next dev`; `start` serves the static export. */
  mode: "start" | "dev";
  /** 0 picks a free port (used by tests). */
  port: number;
  dataDir: string;
  dbPath: string;
  /** Where `next dev` listens, in dev mode. */
  devProxy: URL | null;
}

export interface Environment {
  platform: NodeJS.Platform;
  home: string;
  cwd: string;
  env: Record<string, string | undefined>;
}

/**
 * The per-user app-data folder, outside the repo, so deleting or recloning
 * the checkout never deletes training data.
 */
export function defaultDataDir({ platform, home, env }: Omit<Environment, "cwd">): string {
  switch (platform) {
    case "win32":
      return win32.join(env.APPDATA || win32.join(home, "AppData", "Roaming"), "GymOs");
    case "darwin":
      return posix.join(home, "Library", "Application Support", "GymOs");
    default:
      return posix.join(env.XDG_DATA_HOME || posix.join(home, ".local", "share"), "gymos");
  }
}

/**
 * Reads the server's settings from the environment:
 * - `GYMOS_PORT`: port on 127.0.0.1 (default 4321; 4322 in dev).
 * - `GYMOS_DATA_DIR`: folder holding the database (default: the OS
 *   app-data folder; in dev, a sibling `-dev` folder so dev never touches
 *   real data).
 * - `GYMOS_DEV_PROXY`: set by `pnpm dev` to the `next dev` URL.
 */
export function loadConfig(environment: Environment): ServerConfig {
  const { platform, cwd, env } = environment;
  const path = platform === "win32" ? win32 : posix;

  const devProxy = env.GYMOS_DEV_PROXY ? parseDevProxy(env.GYMOS_DEV_PROXY) : null;
  const mode = devProxy ? "dev" : "start";

  const port = env.GYMOS_PORT ? parsePort(env.GYMOS_PORT) : mode === "dev" ? DEFAULT_DEV_PORT : DEFAULT_PORT;

  const dataDir = env.GYMOS_DATA_DIR
    ? path.resolve(cwd, env.GYMOS_DATA_DIR)
    : `${defaultDataDir(environment)}${mode === "dev" ? "-dev" : ""}`;

  return { mode, port, dataDir, dbPath: path.join(dataDir, DB_FILE_NAME), devProxy };
}

function parsePort(value: string): number {
  const port = Number(value);
  if (!/^\d+$/.test(value) || port > 65535) {
    throw new ConfigError(`GYMOS_PORT must be a port number from 0 to 65535, not "${value}".`);
  }
  return port;
}

function parseDevProxy(value: string): URL {
  let url: URL;
  try {
    url = new URL(value);
  } catch {
    throw new ConfigError(`GYMOS_DEV_PROXY must be a URL, not "${value}".`);
  }
  if (url.protocol !== "http:" || !["127.0.0.1", "localhost"].includes(url.hostname)) {
    throw new ConfigError(`GYMOS_DEV_PROXY must point at this machine over http, not "${value}".`);
  }
  return url;
}
