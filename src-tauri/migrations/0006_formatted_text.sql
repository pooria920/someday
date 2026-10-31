ALTER TABLE items ADD COLUMN formatted_html TEXT
    CHECK (formatted_html IS NULL OR (type IN ('text', 'link') AND length(CAST(formatted_html AS BLOB)) <= 4194304));
