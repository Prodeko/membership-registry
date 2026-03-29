CREATE INDEX IF NOT EXISTS idx_application_user_id ON Application (user_id);
CREATE INDEX IF NOT EXISTS idx_application_status ON Application (status);
CREATE INDEX IF NOT EXISTS idx_savedfilter_owner_user_id ON SavedFilter (owner_user_id);
