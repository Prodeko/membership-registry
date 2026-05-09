ALTER TABLE AttributeDefinition
    ADD COLUMN default_value TEXT NULL CHECK (default_value <> '');
