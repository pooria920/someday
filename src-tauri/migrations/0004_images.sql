CREATE TABLE items_v4 (
    id TEXT PRIMARY KEY NOT NULL,
    type TEXT NOT NULL CHECK (type IN ('link', 'text', 'image')),
    content TEXT,
    title TEXT,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'completed')),
    created_at TEXT NOT NULL,
    completed_at TEXT,
    category_id TEXT REFERENCES categories(id) ON DELETE SET NULL,
    image_original BLOB,
    image_thumbnail BLOB,
    image_mime TEXT,
    image_width INTEGER,
    image_height INTEGER,
    image_hash BLOB,
    CHECK ((status = 'pending' AND completed_at IS NULL)
        OR (status = 'completed' AND completed_at IS NOT NULL)),
    CHECK (
        (type IN ('link', 'text')
            AND content IS NOT NULL AND length(trim(content)) > 0
            AND image_original IS NULL AND image_thumbnail IS NULL
            AND image_mime IS NULL AND image_width IS NULL
            AND image_height IS NULL AND image_hash IS NULL)
        OR
        (type = 'image' AND content IS NULL
            AND title IS NOT NULL AND length(trim(title)) > 0
            AND typeof(image_original) = 'blob' AND length(image_original) BETWEEN 1 AND 20971520
            AND typeof(image_thumbnail) = 'blob' AND length(image_thumbnail) BETWEEN 1 AND 524288
            AND image_mime IS NOT NULL AND image_mime IN ('image/png', 'image/jpeg', 'image/webp')
            AND typeof(image_width) = 'integer' AND image_width BETWEEN 1 AND 16384
            AND typeof(image_height) = 'integer' AND image_height BETWEEN 1 AND 16384
            AND image_width * image_height <= 40000000
            AND typeof(image_hash) = 'blob' AND length(image_hash) = 32)
    )
);
INSERT INTO items_v4 (id, type, content, title, status, created_at, completed_at, category_id)
SELECT id, type, content, title, status, created_at, completed_at, category_id FROM items;
DROP TABLE items;
ALTER TABLE items_v4 RENAME TO items;
CREATE INDEX items_status_created_at ON items(status, created_at DESC);
CREATE INDEX items_category_id ON items(category_id);
CREATE INDEX items_image_hash ON items(image_hash) WHERE type = 'image';
