import type { MetadataRoute } from "next";

// output: 'export' requires this explicitly, even though the docs call
// manifest routes cached by default. The export writes out/manifest.webmanifest.
export const dynamic = "force-static";

export default function manifest(): MetadataRoute.Manifest {
  return {
    id: "/",
    name: "GymOs",
    short_name: "GymOs",
    description: "Offline-first lifting analytics. Your data lives on your devices.",
    start_url: "/",
    scope: "/",
    display: "standalone",
    background_color: "#111827",
    theme_color: "#111827",
    icons: [
      { src: "/icons/icon-192.png", sizes: "192x192", type: "image/png", purpose: "any" },
      { src: "/icons/icon-512.png", sizes: "512x512", type: "image/png", purpose: "any" },
      { src: "/icons/icon-maskable-512.png", sizes: "512x512", type: "image/png", purpose: "maskable" },
    ],
  };
}
