-- Lets members give one free-text value outside allowed_values (an "other"
-- choice). Only meaningful when allowed_values is set; the service rejects it
-- otherwise.
ALTER TABLE AttributeDefinition
    ADD COLUMN allow_other BOOLEAN NOT NULL DEFAULT FALSE;
