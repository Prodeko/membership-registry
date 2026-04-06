CREATE TABLE MarketingTag (
    label         TEXT PRIMARY KEY,
    name_en       TEXT NOT NULL,
    name_fi       TEXT NOT NULL,
    desc_en       TEXT NOT NULL,
    desc_fi       TEXT NOT NULL,
    display_order INTEGER NOT NULL,
    auto_apply    BOOLEAN NOT NULL
);

CREATE INDEX idx_marketing_tag_display_order ON MarketingTag (display_order, label);
