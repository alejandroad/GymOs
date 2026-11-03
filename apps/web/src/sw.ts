/// <reference lib="webworker" />
import { Serwist, type PrecacheEntry, type SerwistGlobalConfig } from "serwist";

// Built by `serwist build` (serwist.config.mjs) after `next build`, into
// out/sw.js. It precaches the whole static export: HTML, RSC payloads, JS and
// CSS chunks, the store worker and the SQLite .wasm, so the app loads and the
// local database works with no network after one online visit.

declare global {
  interface WorkerGlobalScope extends SerwistGlobalConfig {
    // Replaced at build time with the precache manifest.
    __SW_MANIFEST: (PrecacheEntry | string)[] | undefined;
  }
}

declare const self: ServiceWorkerGlobalScope;

const serwist = new Serwist({
  precacheEntries: self.__SW_MANIFEST ?? [],
  precacheOptions: {
    // Serve /spike from spike.html and / from index.html, as static hosts do.
    cleanURLs: true,
    directoryIndex: "index.html",
    cleanupOutdatedCaches: true,
  },
  // Update strategy: a new version installs in the background and waits.
  // It takes over once every GymOs tab and window has closed, so the next
  // launch runs it. It never swaps code under an open session or reloads it.
  skipWaiting: false,
  // Lets the very first install control the page that registered it, so
  // offline works after one visit. Updates still wait (see skipWaiting).
  clientsClaim: true,
  navigationPreload: false,
  disableDevLogs: true,
});

// Worker scripts need special handling. Turbopack starts each worker with
// its bootstrap config in the URL fragment (`#params=...`). When a service
// worker answers with a cached Response, the browser takes the worker's
// location from that response's URL, which has no fragment, and the
// bootstrap fails ("Missing worker bootstrap config"). A fresh Response has
// no URL, so the worker keeps the URL it was requested with.
// Registered before Serwist's listeners so it answers first.
self.addEventListener("fetch", (event) => {
  if (event.request.destination !== "worker") return;
  event.respondWith(
    (async () => {
      const url = new URL(event.request.url);
      const cached = await serwist.matchPrecache(url.origin + url.pathname);
      const response = cached ?? (await fetch(event.request));
      return new Response(response.body, {
        status: response.status,
        statusText: response.statusText,
        headers: response.headers,
      });
    })(),
  );
});

serwist.addEventListeners();
