CREATE TABLE EmailTemplate (
    name text PRIMARY KEY,
    subject text NOT NULL,
    body_html text NOT NULL
);

ALTER TABLE ApplicationTargetableRole
    ADD COLUMN approved_email_template text REFERENCES EmailTemplate(name) ON DELETE SET NULL,
    ADD COLUMN rejected_email_template text REFERENCES EmailTemplate(name) ON DELETE SET NULL;
