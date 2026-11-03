import { newId } from "@gymos/schema";
import { count, desc } from "drizzle-orm";

import type { StoreDb } from "./drizzle";
import { spikeNote, type SpikeNote } from "./schema";

/** Spike-only queries over the demo table. */
export async function addNote(db: StoreDb, body: string): Promise<SpikeNote> {
  const note: SpikeNote = { id: newId(), body, createdAt: Date.now() };
  await db.insert(spikeNote).values(note);
  return note;
}

/** Newest first. UUIDv7 IDs break ties within the same millisecond. */
export function listNotes(db: StoreDb, limit: number): Promise<SpikeNote[]> {
  return db.select().from(spikeNote).orderBy(desc(spikeNote.createdAt), desc(spikeNote.id)).limit(limit);
}

export async function countNotes(db: StoreDb): Promise<number> {
  const [row] = await db.select({ n: count() }).from(spikeNote);
  return row?.n ?? 0;
}
