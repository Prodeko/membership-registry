-- Re-add columns to EmailTemplate
ALTER TABLE EmailTemplate ADD COLUMN subject TEXT NOT NULL DEFAULT '';
ALTER TABLE EmailTemplate ADD COLUMN body_html TEXT NOT NULL DEFAULT '';

-- Restore Finnish translations back to EmailTemplate
UPDATE EmailTemplate
SET subject = t.subject, body_html = t.body_html
FROM EmailTemplateTranslation t
WHERE EmailTemplate.name = t.template_name AND t.locale = 'fi';

-- Drop translation table
DROP TABLE EmailTemplateTranslation;

-- Remove language from Member
ALTER TABLE Member DROP COLUMN language;
