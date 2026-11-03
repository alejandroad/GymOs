// Spike-only queries over the demo table.
import { count, desc } from "drizzle-orm";

import type { StoreDb } from "./drizzle";
import { spikeNote, type SpikeNote } from "./schema";

/**
 * Idempotent: the caller generates the UUIDv7, so a request that is retried
 * (for example, resent to a new leader tab) inserts the note only once.
 */
export async function addNote(db: StoreDb, note: SpikeNote): Promise<SpikeNote> {
  await db.insert(spikeNote).values(note).onConflictDoNothing();
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
