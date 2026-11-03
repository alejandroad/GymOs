# PWA and service worker

How GymOs loads with no network, how a new version reaches users, and what any host must send. Applies from E0-T4.

## 1. What is precached

`pnpm build` runs `next build` (static export to `apps/web/out/`), then `serwist build serwist.config.mjs`. The Serwist CLI:

1. globs `out/` for `html, txt, js, css, wasm, webmanifest, png, svg, ico, woff2`;
2. bundles `apps/web/src/sw.ts` with esbuild;
3. injects the precache manifest and writes `out/sw.js`.

That covers every route's HTML, the RSC payloads (`*.txt`) used for client-side navigation, every JS and CSS chunk, the store worker chunk, the SQLite `.wasm`, the manifest and the icons: about 39 URLs and 1.9 MB today. Files under `_next/static/` have content hashes in their names, so they are precached without a revision. Everything else carries a revision hash and is re-fetched only when its content changes.

Ignored on purpose: copies Turbopack emits next to the real chunks (`_next/static/media/*.ts`, sqlite-wasm's `sqlite3-worker1*` and `sqlite3-opfs-async-proxy*`). Nothing loads them at runtime.

**Why the Serwist CLI and not `@serwist/next`:** the Next plugin has no documented path for `output: 'export'`. The CLI works on whatever is in `out/` and depends on no Next.js build internals, so the service worker survives framework upgrades and would carry over unchanged to a Vite SPA.

## 2. Update strategy

| Setting | Value | Effect |
|---|---|---|
| `skipWaiting` | `false` | A new version installs in the background and **waits**. It activates once every GymOs tab and window has closed, so it runs on the next launch. A session never has its code swapped or gets reloaded under it. |
| `clientsClaim` | `true` | The very first install takes control of the page that registered it, so offline works after one online visit without a second reload. It has no effect on updates, which only activate when no client is open. |
| `cleanupOutdatedCaches` | `true` | Old precaches are deleted when a new version activates. |
| `updateViaCache` (registration) | `"none"` | The browser bypasses the HTTP cache when it checks `sw.js` for updates. |

"Next load" here means the next launch after all GymOs windows are closed. A plain reload of one tab doesn't activate a waiting worker, because the old page stays a client during the navigation. That's browser behavior, and it's what prevents surprise reloads. E2 can add an "Update ready" notice with an explicit "Restart now" button if users need it sooner.

The service worker registers in production builds only. `next dev` doesn't emit `sw.js`.

## 3. Turbopack workers and service workers

Turbopack starts each Web Worker with its bootstrap config in the URL fragment (`…/turbopack-worker-*.js#params=…`). When a service worker answers that request with a cached `Response`, the browser sets the worker's location to the response's URL. That URL has no fragment, so the bootstrap fails with `Missing worker bootstrap config`, and only on the **second** load (the first visit isn't controlled yet).

`src/sw.ts` answers `destination === "worker"` requests itself, before Serwist's handlers. It reads the script from the precache and returns it in a **fresh** `Response`. A response with no URL leaves the worker on the URL it was requested with, fragment included. Keep this in mind for any future worker (importers, engines).

## 4. Headers a host must send

| Path | Header | Why |
|---|---|---|
| `/sw.js` | `Cache-Control: no-cache` (or `max-age=0, must-revalidate`) | The browser must re-check the worker on every navigation, or users can be stuck on an old version. |
| `/_next/static/*` | `Cache-Control: public, max-age=31536000, immutable` | Content-hashed; safe to cache forever. |
| `*.wasm` | `Content-Type: application/wasm` | Needed for streaming WebAssembly compilation. |

No COOP/COEP headers are needed: the `opfs-sahpool` VFS doesn't use `SharedArrayBuffer`. Clean URLs (`/spike` → `spike.html`) must also work, as they do by default on the hosts below.

### Local preview (`serve`)

`apps/web/serve.json`, passed with `--config ../serve.json` by `pnpm preview`. It sends the first two headers; `serve` sets the wasm MIME type and clean URLs on its own.

### Vercel

Expected, **not yet verified** (that's E0-T6): Vercel serves a static export's un-hashed files with `max-age=0, must-revalidate`, serves `_next/static` as immutable, and sets the wasm MIME type, so no config file is needed. If the T6 check of the response headers shows otherwise, add `apps/web/vercel.json`:

```json
{
  "headers": [
    { "source": "/sw.js", "headers": [{ "key": "Cache-Control", "value": "no-cache" }] },
    {
      "source": "/_next/static/(.*)",
      "headers": [{ "key": "Cache-Control", "value": "public, max-age=31536000, immutable" }]
    }
  ]
}
```

### Netlify and Cloudflare Pages

Both read a `_headers` file at the publish root. Put it in `apps/web/public/_headers` so the export copies it into `out/`:

```
/sw.js
  Cache-Control: no-cache

/_next/static/*
  Cache-Control: public, max-age=31536000, immutable
```

### nginx

```nginx
root /var/www/gymos/out;

location = /sw.js {
  add_header Cache-Control "no-cache";
}

location /_next/static/ {
  add_header Cache-Control "public, max-age=31536000, immutable";
}

location / {
  try_files $uri $uri.html $uri/ =404;
}

# mime.types normally includes this; add it if yours doesn't.
types { application/wasm wasm; }
```

## 5. Verifying

Against `pnpm build && pnpm preview`, using the runtime criteria in [`platform-support.md`](platform-support.md):

- **R-8:** load `/` and `/spike` once online, stop the preview server (or tick DevTools → Network → Offline), and reload. Both pages load and `/spike` reads and writes.
- **R-9:** the browser offers an install.

Stopping the server is the stronger test, because DevTools' offline mode doesn't always cover the service worker's own requests.
