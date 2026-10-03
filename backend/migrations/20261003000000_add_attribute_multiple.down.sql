-- Lossy: multi-valued attributes keep only their first value.
ALTER TABLE MemberAttribute ADD COLUMN value TEXT;
UPDATE MemberAttribute SET value = value_list[1];
ALTER TABLE MemberAttribute
    ALTER COLUMN value SET NOT NULL,
    ADD CONSTRAINT memberattribute_value_check CHECK (value <> ''),
    DROP COLUMN value_list;

ALTER TABLE AttributeDefinition DROP COLUMN multiple;
