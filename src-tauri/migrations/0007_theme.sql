ALTER TABLE shortcut_settings ADD COLUMN theme TEXT CHECK(theme IN ('light','dark'));
