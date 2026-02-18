-- Dev data seed script
-- Run with: docker compose exec -T membership-postgresd psql -U membership -d membership < backend/dev-data.sql

-- Role
INSERT INTO role (name, description, color)
VALUES ('membership', '', '')
ON CONFLICT (name) DO NOTHING;

-- Email templates
INSERT INTO emailtemplate (name, subject, body_html)
VALUES (
    'membership_approved',
    'Prodeko application approved',
    '<h1>Application approved</h1>
<p>Hi {name}.<br>Your application for the role {role_name} has been approved</p>'
)
ON CONFLICT (name) DO UPDATE SET
    subject = EXCLUDED.subject,
    body_html = EXCLUDED.body_html;

INSERT INTO emailtemplate (name, subject, body_html)
VALUES (
    'membership_rejected',
    'Prodeko application rejected',
    '<h1>Application rejected</h1>
<p>Hi {name}.<br>Your application for the role {role_name} has been rejected</p>'
)
ON CONFLICT (name) DO UPDATE SET
    subject = EXCLUDED.subject,
    body_html = EXCLUDED.body_html;

-- Application targetable role
INSERT INTO applicationtargetablerole (role_name, valid_until, active, payment_link, optional_roles, approved_email_template, rejected_email_template)
VALUES (
    'membership',
    '2026-12-30',
    true,
    'https://buy.stripe.com/test_3cIeV6b1Y2Ec4Ck2lx4ow00',
    NULL,
    'membership_approved',
    'membership_rejected'
)
ON CONFLICT (role_name, valid_until) DO UPDATE SET
    active = EXCLUDED.active,
    payment_link = EXCLUDED.payment_link,
    optional_roles = EXCLUDED.optional_roles,
    approved_email_template = EXCLUDED.approved_email_template,
    rejected_email_template = EXCLUDED.rejected_email_template;
