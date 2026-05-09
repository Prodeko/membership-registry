CREATE TABLE ApplicationTargetableRoleAttribute (
    role_name TEXT NOT NULL,
    valid_until DATE NOT NULL,
    attribute_name TEXT NOT NULL REFERENCES AttributeDefinition(name) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    PRIMARY KEY (role_name, valid_until, attribute_name),
    FOREIGN KEY (role_name, valid_until)
        REFERENCES ApplicationTargetableRole(role_name, valid_until)
        ON DELETE CASCADE
);

CREATE INDEX ApplicationTargetableRoleAttribute_lookup_idx
    ON ApplicationTargetableRoleAttribute (role_name, valid_until, position);
