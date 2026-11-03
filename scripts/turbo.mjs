// Runs Turborepo with every toolchain telemetry opt-out set, so neither
// Turbo nor Next.js phones home from a developer's or user's machine
// (handoff principle 5). Root package.json scripts go through here.
// Usage: node scripts/turbo.mjs run build
import { spawn } from "node:child_process";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";

export const NO_TELEMETRY = {
  TURBO_TELEMETRY_DISABLED: "1",
  NEXT_TELEMETRY_DISABLED: "1",
  DO_NOT_TRACK: "1",
};

const require = createRequire(import.meta.url);
const turboPackage = require.resolve("turbo/package.json");
const turboBin = join(dirname(turboPackage), JSON.parse(readFileSync(turboPackage, "utf8")).bin.turbo);

/** Runs turbo with `args` and resolves with its exit code. */
export function turbo(args) {
  // Run turbo's JS launcher with this Node: no shell and no .cmd shim, so it
  // behaves the same on Windows, macOS and Linux.
  const child = spawn(process.execPath, [turboBin, ...args], {
    stdio: "inherit",
    env: { ...process.env, ...NO_TELEMETRY },
  });
  // Ctrl+C reaches turbo directly (same console); wait for it to clean up.
  const ignore = () => {};
  process.on("SIGINT", ignore);
  return new Promise((resolve) => {
    child.on("exit", (code, signal) => {
      process.off("SIGINT", ignore);
      resolve(code ?? (signal ? 1 : 0));
    });
  });
}

if (import.meta.main) process.exit(await turbo(process.argv.slice(2)));
