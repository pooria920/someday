ALTER TABLE shortcut_settings ADD COLUMN calendar_type TEXT NOT NULL DEFAULT 'persian' CHECK (calendar_type IN ('persian', 'gregory'));
