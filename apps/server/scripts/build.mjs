// Bundles the server and the workspace packages it imports into one ESM
// file that plain `node` runs. Workspace packages export TypeScript source
// with extensionless imports, which Node's type stripping can't resolve.
import { fileURLToPath } from "node:url";

import { build } from "esbuild";

export const buildOptions = {
  entryPoints: [fileURLToPath(new URL("../src/main.ts", import.meta.url))],
  outfile: fileURLToPath(new URL("../dist/server.mjs", import.meta.url)),
  bundle: true,
  platform: "node",
  format: "esm",
  target: "node26",
  sourcemap: true,
  logLevel: "warning",
};

if (import.meta.main) {
  await build(buildOptions);
  console.log("Bundled the server into dist/server.mjs");
}
