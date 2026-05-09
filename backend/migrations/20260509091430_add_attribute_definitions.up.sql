CREATE TYPE attribute_editable_by AS ENUM ('admin', 'user', 'both');

CREATE TABLE AttributeDefinition (
    name             TEXT PRIMARY KEY,
    description      TEXT,
    allowed_values   TEXT[],
    sync_to_keycloak BOOLEAN NOT NULL,
    editable_by      attribute_editable_by NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
