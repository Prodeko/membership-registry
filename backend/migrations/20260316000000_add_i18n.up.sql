-- Add language preference to Member (source of truth, synced to Keycloak)
ALTER TABLE Member ADD COLUMN language TEXT NOT NULL DEFAULT 'fi';

-- Email template translations (child of EmailTemplate)
CREATE TABLE EmailTemplateTranslation (
    template_name TEXT NOT NULL REFERENCES EmailTemplate(name) ON DELETE CASCADE ON UPDATE CASCADE,
    locale TEXT NOT NULL,
    subject TEXT NOT NULL,
    body_html TEXT NOT NULL,
    PRIMARY KEY (template_name, locale)
);

-- Migrate existing template content to Finnish translations
INSERT INTO EmailTemplateTranslation (template_name, locale, subject, body_html)
SELECT name, 'fi', subject, body_html FROM EmailTemplate;

-- Drop old columns now that content lives in translation table
ALTER TABLE EmailTemplate DROP COLUMN subject;
ALTER TABLE EmailTemplate DROP COLUMN body_html;
