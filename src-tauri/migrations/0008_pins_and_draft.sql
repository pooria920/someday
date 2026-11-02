ALTER TABLE items ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0 CHECK(pinned IN (0,1));
CREATE TABLE note_draft (
    id INTEGER PRIMARY KEY CHECK(id = 1),
    content TEXT NOT NULL CHECK(typeof(content) = 'text' AND length(CAST(content AS BLOB)) <= 4194304)
);
INSERT INTO note_draft(id, content) VALUES (1, '');
