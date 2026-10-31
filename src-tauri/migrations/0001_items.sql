CREATE TABLE items (
    id TEXT PRIMARY KEY NOT NULL,
    type TEXT NOT NULL CHECK (type IN ('link', 'text')),
    content TEXT NOT NULL CHECK (length(trim(content)) > 0),
    title TEXT,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'completed')),
    created_at TEXT NOT NULL,
    completed_at TEXT,
    CHECK ((status = 'pending' AND completed_at IS NULL)
        OR (status = 'completed' AND completed_at IS NOT NULL))
);
CREATE INDEX items_status_created_at ON items(status, created_at DESC);
