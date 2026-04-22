CREATE TABLE RoleGroup (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    keycloak_group_id TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE RoleGroupRole (
    group_id UUID NOT NULL REFERENCES RoleGroup(id) ON DELETE CASCADE,
    role_name TEXT NOT NULL REFERENCES Role(name) ON DELETE CASCADE,
    PRIMARY KEY (group_id, role_name)
);

CREATE TABLE RoleGroupMember (
    group_id UUID NOT NULL REFERENCES RoleGroup(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES Member(user_id) ON DELETE CASCADE,
    valid_from DATE NOT NULL DEFAULT CURRENT_DATE,
    valid_until DATE,
    keycloak_removed_at TIMESTAMPTZ,
    PRIMARY KEY (group_id, user_id, valid_from)
);

CREATE INDEX idx_role_group_member_user ON RoleGroupMember(user_id);
CREATE INDEX idx_role_group_member_expiry ON RoleGroupMember(valid_until)
    WHERE keycloak_removed_at IS NULL;
