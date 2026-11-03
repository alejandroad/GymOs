// `pnpm dev` for the server: rebuilds the bundle on every change and
// restarts it. In dev the server proxies the UI (and its HMR websocket) to
// `next dev`, so the page and /api/rpc share one origin, as in production.
import { spawn } from "node:child_process";

import { context } from "esbuild";

import { buildOptions } from "./build.mjs";

const env = {
  ...process.env,
  GYMOS_DEV_PROXY: process.env.GYMOS_DEV_PROXY ?? "http://127.0.0.1:3000",
};

let child = null;
/** Serializes restarts: the old server must exit (and release the database lock) first. */
let restarting = Promise.resolve();

function stopChild() {
  if (!child || child.exitCode !== null) return Promise.resolve();
  const exited = new Promise((resolve) => child.once("exit", resolve));
  child.kill();
  return exited;
}

function restart() {
  restarting = restarting.then(async () => {
    await stopChild();
    child = spawn(process.execPath, ["--enable-source-maps", buildOptions.outfile], { stdio: "inherit", env });
  });
}

const ctx = await context({
  ...buildOptions,
  plugins: [
    {
      name: "restart-server",
      setup(build) {
        build.onEnd((result) => {
          if (result.errors.length === 0) restart();
        });
      },
    },
  ],
});
await ctx.watch();

for (const signal of ["SIGINT", "SIGTERM", "SIGBREAK"]) {
  process.on(signal, async () => {
    await ctx.dispose();
    await stopChild();
    process.exit(0);
  });
}
