CREATE TABLE item_reminders (
    item_id TEXT PRIMARY KEY NOT NULL REFERENCES items(id) ON DELETE CASCADE,
    due_at INTEGER NOT NULL CHECK (typeof(due_at) = 'integer' AND due_at > 0 AND due_at <= 253402300799999)
);
CREATE INDEX item_reminders_due_at ON item_reminders(due_at, item_id);
