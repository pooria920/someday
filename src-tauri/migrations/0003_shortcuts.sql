CREATE TABLE shortcut_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    capture TEXT NOT NULL,
    toggle TEXT NOT NULL CHECK (toggle <> capture)
);
INSERT INTO shortcut_settings (id, capture, toggle)
VALUES (1, 'Ctrl+Shift+S', 'Ctrl+Shift+Space');
