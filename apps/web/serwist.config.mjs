// @ts-check
// Serwist CLI config: `serwist build` bundles src/sw.ts with esbuild and
// injects a precache manifest globbed from the static export. It runs after
// `next build` (see the build script), so out/ already exists. No Next.js
// plugin is involved, and nothing here depends on the host.

/** @type {import("@serwist/cli").BuildOptions} */
const config = {
  swSrc: "src/sw.ts",
  swDest: "out/sw.js",
  globDirectory: "out",
  globPatterns: ["**/*.{html,txt,js,css,wasm,webmanifest,png,svg,ico,woff2}"],
  globIgnores: [
    "sw.js",
    // Copies Turbopack emits next to the bundled chunks; nothing loads them.
    "_next/static/media/*.ts",
    "_next/static/media/sqlite3-worker1*",
    "_next/static/media/sqlite3-opfs-async-proxy*",
  ],
  // Hashed file names already change with their content.
  dontCacheBustURLsMatching: /^_next\/static\//,
  // The SQLite .wasm is ~0.9 MB; keep headroom without precaching anything huge.
  maximumFileSizeToCacheInBytes: 5 * 1024 * 1024,
};

export default config;
