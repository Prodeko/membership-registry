ALTER TABLE AttributeDefinition
    ADD COLUMN multiple BOOLEAN NOT NULL DEFAULT FALSE;

-- A member holds one or more values per attribute. Single-valued attributes
-- (multiple = false) always have exactly one element; the service enforces
-- that, the table only guarantees non-empty and no empty strings.
ALTER TABLE MemberAttribute ADD COLUMN value_list TEXT[];
UPDATE MemberAttribute SET value_list = ARRAY[value];
ALTER TABLE MemberAttribute
    ALTER COLUMN value_list SET NOT NULL,
    DROP COLUMN value,
    ADD CONSTRAINT memberattribute_value_list_check
        CHECK (cardinality(value_list) > 0 AND array_position(value_list, '') IS NULL);
