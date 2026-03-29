DROP INDEX IF EXISTS idx_role_renewal_user_role;
DROP INDEX IF EXISTS idx_role_renewal_status_old_valid_until;
DROP TABLE IF EXISTS RoleRenewal;
DROP TYPE IF EXISTS renewal_status;

ALTER TABLE Role
  DROP COLUMN IF EXISTS renewable,
  DROP COLUMN IF EXISTS renewal_payment_link,
  DROP COLUMN IF EXISTS renewal_period_months,
  DROP COLUMN IF EXISTS renewal_email_template,
  DROP COLUMN IF EXISTS renewal_notification_days;
