-- Per-role, per-locale renewal banner texts
CREATE TABLE RoleRenewalPrompt (
  role_name    TEXT NOT NULL REFERENCES Role(name) ON DELETE CASCADE,
  locale       TEXT NOT NULL,
  title        TEXT NOT NULL,
  body         TEXT NOT NULL,
  button_label TEXT NOT NULL,
  PRIMARY KEY (role_name, locale)
);
