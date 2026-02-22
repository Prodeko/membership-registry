-- Drop auth provider table
DROP INDEX IF EXISTS idx_user_auth_provider_user;
DROP INDEX IF EXISTS idx_user_auth_provider_lookup;
DROP TABLE IF EXISTS UserAuthProvider;

-- Remove foreign key constraints
ALTER TABLE SavedFilter DROP CONSTRAINT IF EXISTS fk_saved_filter_owner;
ALTER TABLE Application DROP CONSTRAINT IF EXISTS fk_application_user_id;
