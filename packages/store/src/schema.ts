import { index, integer, sqliteTable, text } from "drizzle-orm/sqlite-core";

/**
 * Spike-only demo table. IDs are client-generated UUIDv7 (handoff rule 13),
 * never database serials. Replaced by the product schema in E1/E3.
 */
export const spikeNote = sqliteTable(
  "spike_note",
  {
    id: text("id").primaryKey(),
    body: text("body").notNull(),
    /** Unix epoch milliseconds. */
    createdAt: integer("created_at").notNull(),
  },
  (table) => [index("spike_note_created_at_idx").on(table.createdAt)],
);

export type SpikeNote = typeof spikeNote.$inferSelect;
