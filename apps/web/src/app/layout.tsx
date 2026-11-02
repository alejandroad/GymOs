import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "GymOs",
  description:
    "A smart lifting diary that makes progressive overload visible and actionable.",
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
