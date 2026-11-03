CREATE TABLE `spike_note` (
	`id` text PRIMARY KEY NOT NULL,
	`body` text NOT NULL,
	`created_at` integer NOT NULL
);
--> statement-breakpoint
CREATE INDEX `spike_note_created_at_idx` ON `spike_note` (`created_at`);