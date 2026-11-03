// `pnpm start`: builds the UI and the server (cached after the first run),
// then runs the server in this process, so Ctrl+C stops it cleanly.
import { join } from "node:path";
import { pathToFileURL } from "node:url";

import { turbo } from "./turbo.mjs";

console.log("Building GymOs. The first run takes a minute; later runs reuse the build.");
const status = await turbo([
  "run",
  "build",
  "--filter=@gymos/server...",
  "--output-logs=errors-only",
  "--log-prefix=none",
]);
if (status !== 0) {
  console.error("\nThe build failed (see above), so GymOs didn't start.");
  process.exit(status);
}

await import(pathToFileURL(join(import.meta.dirname, "..", "apps", "server", "dist", "server.mjs")).href);
