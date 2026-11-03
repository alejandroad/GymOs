"use client";

import { useEffect } from "react";

/**
 * Registers out/sw.js. Production builds only: `next dev` doesn't emit the
 * service worker, and a stale one would serve old dev assets.
 *
 * updateViaCache: "none" makes the browser bypass the HTTP cache when it
 * checks sw.js for updates, so a misconfigured Cache-Control on a host can't
 * pin an old version. Hosts should still serve sw.js with no-cache.
 */
export function ServiceWorkerRegistration() {
  useEffect(() => {
    if (process.env.NODE_ENV !== "production" || !("serviceWorker" in navigator)) return;
    navigator.serviceWorker
      .register("/sw.js", { scope: "/", updateViaCache: "none" })
      .catch((error: unknown) => console.error("Service worker registration failed", error));
  }, []);
  return null;
}
