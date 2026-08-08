DROP INDEX IF EXISTS idx_role_renewal_pending_unique;

ALTER TABLE RoleRenewal
  ADD COLUMN notified_30d BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN notified_7d BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN notified_1d BOOLEAN NOT NULL DEFAULT FALSE;

UPDATE RoleRenewal SET
  notified_30d = 30 = ANY(notified_days),
  notified_7d  = 7  = ANY(notified_days),
  notified_1d  = 1  = ANY(notified_days);

ALTER TABLE RoleRenewal DROP COLUMN notified_days;

ALTER TABLE Role
  DROP COLUMN IF EXISTS renewal_window_days,
  DROP COLUMN IF EXISTS grace_period_days;
