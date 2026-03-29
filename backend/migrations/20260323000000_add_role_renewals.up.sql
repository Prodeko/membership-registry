-- Add renewal configuration to Role table
ALTER TABLE Role
  ADD COLUMN renewable BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN renewal_payment_link TEXT,
  ADD COLUMN renewal_period_months INTEGER,
  ADD COLUMN renewal_email_template TEXT REFERENCES EmailTemplate(name) ON DELETE SET NULL,
  ADD COLUMN renewal_notification_days INTEGER[] NOT NULL DEFAULT '{30, 7, 1}';

-- Renewal status enum
CREATE TYPE renewal_status AS ENUM ('pending', 'paid', 'expired');

-- Track pending/completed role renewals
CREATE TABLE IF NOT EXISTS RoleRenewal (
  renewal_id      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id         UUID NOT NULL REFERENCES Member(user_id) ON DELETE CASCADE,
  role_name       TEXT NOT NULL REFERENCES Role(name) ON DELETE CASCADE,
  old_valid_from  DATE NOT NULL,
  old_valid_until DATE NOT NULL,
  new_valid_from  DATE NOT NULL,
  new_valid_until DATE NOT NULL,
  status          renewal_status NOT NULL DEFAULT 'pending',
  stripe_payment_id TEXT,
  created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  notified_30d    BOOLEAN NOT NULL DEFAULT FALSE,
  notified_7d     BOOLEAN NOT NULL DEFAULT FALSE,
  notified_1d     BOOLEAN NOT NULL DEFAULT FALSE,
  FOREIGN KEY (user_id, role_name, old_valid_from)
    REFERENCES RoleMember(user_id, role_name, valid_from) ON DELETE CASCADE
);

-- Index for scheduler queries: find pending renewals for expiring roles
CREATE INDEX idx_role_renewal_status_old_valid_until
  ON RoleRenewal (status, old_valid_until);

-- Index for webhook lookup by renewal_id (PK already covers this)
-- Index for finding renewals by user+role
CREATE INDEX idx_role_renewal_user_role
  ON RoleRenewal (user_id, role_name, old_valid_from);
