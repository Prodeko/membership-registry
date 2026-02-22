ALTER TABLE ApplicationTargetableRole
    DROP COLUMN approved_email_template,
    DROP COLUMN rejected_email_template;

DROP TABLE EmailTemplate;
