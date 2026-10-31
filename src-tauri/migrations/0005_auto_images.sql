ALTER TABLE shortcut_settings ADD COLUMN auto_save_images INTEGER NOT NULL DEFAULT 0
    CHECK (auto_save_images IN (0, 1));
