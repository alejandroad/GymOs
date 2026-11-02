import { ping } from "@gymos/db";
import { connection } from "next/server";

import { getDb } from "@/lib/db";

const noStore = { "Cache-Control": "no-store" };

export async function GET() {
  // Opt out of prerendering: with Cache Components, a GET handler that
  // touches no request data would otherwise run once at build time.
  await connection();
  const time = new Date().toISOString();

  try {
    await ping(getDb());
    return Response.json({ status: "ok", db: "ok", time }, { headers: noStore });
  } catch (error) {
    // Log the cause server-side only; the response never carries
    // connection details.
    console.error("Health check: database ping failed", error);
    return Response.json(
      { status: "error", db: "error", time },
      { status: 503, headers: noStore },
    );
  }
}
