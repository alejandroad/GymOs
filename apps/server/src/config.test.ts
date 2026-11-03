import { describe, expect, it } from "vitest";

import { ConfigError, defaultDataDir, loadConfig, type Environment } from "./config";

const windows: Environment = {
  platform: "win32",
  home: "C:\\Users\\ana",
  cwd: "C:\\code\\GymOs",
  env: { APPDATA: "C:\\Users\\ana\\AppData\\Roaming" },
};
const mac: Environment = { platform: "darwin", home: "/Users/ana", cwd: "/code/GymOs", env: {} };
const linux: Environment = { platform: "linux", home: "/home/ana", cwd: "/code/GymOs", env: {} };

describe("defaultDataDir", () => {
  it("uses the OS app-data folder, outside the repo", () => {
    expect(defaultDataDir(windows)).toBe("C:\\Users\\ana\\AppData\\Roaming\\GymOs");
    expect(defaultDataDir(mac)).toBe("/Users/ana/Library/Application Support/GymOs");
    expect(defaultDataDir(linux)).toBe("/home/ana/.local/share/gymos");
  });

  it("falls back when APPDATA is unset and honours XDG_DATA_HOME", () => {
    expect(defaultDataDir({ ...windows, env: {} })).toBe("C:\\Users\\ana\\AppData\\Roaming\\GymOs");
    expect(defaultDataDir({ ...linux, env: { XDG_DATA_HOME: "/data" } })).toBe("/data/gymos");
  });
});

describe("loadConfig", () => {
  it("defaults to port 4321 and the app-data database", () => {
    expect(loadConfig(windows)).toEqual({
      mode: "start",
      port: 4321,
      dataDir: "C:\\Users\\ana\\AppData\\Roaming\\GymOs",
      dbPath: "C:\\Users\\ana\\AppData\\Roaming\\GymOs\\gymos.sqlite3",
      devProxy: null,
    });
  });

  it("takes the port and data dir from the environment, resolving a relative dir", () => {
    const config = loadConfig({ ...mac, env: { GYMOS_PORT: "0", GYMOS_DATA_DIR: "data" } });
    expect(config.port).toBe(0);
    expect(config.dbPath).toBe("/code/GymOs/data/gymos.sqlite3");
  });

  it("keeps dev mode on its own port and data folder", () => {
    const config = loadConfig({ ...linux, env: { GYMOS_DEV_PROXY: "http://127.0.0.1:3000" } });
    expect(config.mode).toBe("dev");
    expect(config.port).toBe(4322);
    expect(config.dataDir).toBe("/home/ana/.local/share/gymos-dev");
    expect(config.devProxy?.href).toBe("http://127.0.0.1:3000/");
  });

  it.each(["abc", "-1", "65536", "80.5", " 80"])("rejects GYMOS_PORT=%j", (value) => {
    expect(() => loadConfig({ ...linux, env: { GYMOS_PORT: value } })).toThrow(ConfigError);
  });

  it.each(["not a url", "https://127.0.0.1:3000", "http://example.com:3000"])(
    "rejects GYMOS_DEV_PROXY=%j",
    (value) => {
      expect(() => loadConfig({ ...linux, env: { GYMOS_DEV_PROXY: value } })).toThrow(ConfigError);
    },
  );
});
