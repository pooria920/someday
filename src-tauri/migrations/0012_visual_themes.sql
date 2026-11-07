ALTER TABLE shortcut_settings RENAME COLUMN color_palette TO previous_palette;
ALTER TABLE shortcut_settings ADD COLUMN color_palette TEXT NOT NULL DEFAULT 'lime'
 CHECK(color_palette IN ('lime','violet','ocean','rose','amber','notebook','terminal'));
UPDATE shortcut_settings SET color_palette=previous_palette;
ALTER TABLE shortcut_settings DROP COLUMN previous_palette;
