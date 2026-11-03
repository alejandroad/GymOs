import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "GymOs",
  description:
    "Offline-first lifting analytics. Your data lives on your devices.",
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
