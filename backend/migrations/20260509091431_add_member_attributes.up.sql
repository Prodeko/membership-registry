CREATE TABLE MemberAttribute (
    user_id          UUID NOT NULL REFERENCES Member(user_id) ON DELETE CASCADE,
    attribute_name   TEXT NOT NULL REFERENCES AttributeDefinition(name) ON DELETE CASCADE,
    value            TEXT NOT NULL CHECK (value <> ''),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, attribute_name)
);

CREATE INDEX MemberAttribute_attribute_name_idx ON MemberAttribute (attribute_name);
