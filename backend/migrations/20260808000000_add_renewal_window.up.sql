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

-- Expire older duplicates so the unique index below can always be created
UPDATE RoleRenewal rr SET status = 'expired'
WHERE rr.status = 'pending'
  AND EXISTS (
    SELECT 1 FROM RoleRenewal newer
    WHERE newer.user_id = rr.user_id
      AND newer.role_name = rr.role_name
      AND newer.old_valid_from = rr.old_valid_from
      AND newer.status = 'pending'
      AND newer.created_at > rr.created_at
  );

-- At most one pending renewal per role membership (concurrent-request guard)
CREATE UNIQUE INDEX idx_role_renewal_pending_unique
  ON RoleRenewal (user_id, role_name, old_valid_from)
  WHERE status = 'pending';
