-- Add foreign key constraints that were missing from initial schema
ALTER TABLE Application
ADD CONSTRAINT fk_application_user_id
FOREIGN KEY (user_id) REFERENCES Member(user_id) ON DELETE CASCADE;

ALTER TABLE SavedFilter
ADD CONSTRAINT fk_saved_filter_owner
FOREIGN KEY (owner_user_id) REFERENCES Member(user_id) ON DELETE CASCADE;

-- Create table to map auth provider IDs to internal user UUIDs
CREATE TABLE UserAuthProvider (
    user_id uuid NOT NULL,
    provider_name text NOT NULL,
    provider_user_id text NOT NULL,
    linked_at timestamptz NOT NULL DEFAULT NOW(),
    metadata jsonb,
    PRIMARY KEY (provider_name, provider_user_id),
    FOREIGN KEY (user_id) REFERENCES Member(user_id) ON DELETE CASCADE,
    CONSTRAINT unique_user_provider UNIQUE (user_id, provider_name)
);

-- Index for fast lookup during authentication
CREATE INDEX idx_user_auth_provider_lookup ON UserAuthProvider(provider_name, provider_user_id);

-- Index for finding all providers for a user
CREATE INDEX idx_user_auth_provider_user ON UserAuthProvider(user_id);
