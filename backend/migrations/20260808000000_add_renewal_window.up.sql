-- Renewal window and post-expiry grace period configuration
ALTER TABLE Role
  ADD COLUMN renewal_window_days INTEGER NOT NULL DEFAULT 30,
  ADD COLUMN grace_period_days INTEGER NOT NULL DEFAULT 0;

-- Track sent reminder offsets per renewal, supporting arbitrary day offsets
ALTER TABLE RoleRenewal
  ADD COLUMN notified_days INTEGER[] NOT NULL DEFAULT '{}';

UPDATE RoleRenewal SET notified_days = ARRAY_REMOVE(ARRAY[
  CASE WHEN notified_30d THEN 30 END,
  CASE WHEN notified_7d THEN 7 END,
  CASE WHEN notified_1d THEN 1 END
], NULL);

ALTER TABLE RoleRenewal
  DROP COLUMN notified_30d,
  DROP COLUMN notified_7d,
  DROP COLUMN notified_1d;

-- At most one pending renewal per role membership (concurrent-request guard)
CREATE UNIQUE INDEX idx_role_renewal_pending_unique
  ON RoleRenewal (user_id, role_name, old_valid_from)
  WHERE status = 'pending';
