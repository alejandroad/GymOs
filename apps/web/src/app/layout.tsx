import type { Metadata, Viewport } from "next";
import "./globals.css";
import { ServiceWorkerRegistration } from "./service-worker-registration";

export const metadata: Metadata = {
  title: "GymOs",
  description: "Offline-first lifting analytics. Your data lives on your devices.",
  applicationName: "GymOs",
  // iOS reads these instead of the manifest for Home Screen apps.
  appleWebApp: {
    capable: true,
    title: "GymOs",
    statusBarStyle: "default",
  },
  icons: {
    icon: [{ url: "/icons/icon-192.png", sizes: "192x192", type: "image/png" }],
    apple: [{ url: "/icons/apple-touch-icon.png", sizes: "180x180", type: "image/png" }],
  },
};

export const viewport: Viewport = {
  themeColor: "#111827",
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html lang="en">
      <body>
        {children}
        <ServiceWorkerRegistration />
      </body>
    </html>
  );
}
