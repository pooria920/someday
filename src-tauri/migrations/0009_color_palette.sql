ALTER TABLE shortcut_settings ADD COLUMN color_palette TEXT NOT NULL DEFAULT 'lime'
  CHECK(color_palette IN ('lime', 'violet', 'ocean', 'rose', 'amber'));
