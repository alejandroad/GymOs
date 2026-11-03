import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Static export: no server runtime, so any static host (or localhost) can
  // serve out/. Route handlers, Server Actions and proxy are unavailable.
  output: "export",
  turbopack: {
    rules: {
      "*.css": {
        loaders: ["@tailwindcss/turbopack"],
        as: "*.css",
      },
    },
  },
};

export default nextConfig;
