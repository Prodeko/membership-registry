-- Values given via "other" stay in MemberAttribute; they now just fall
-- outside allowed_values until an admin cleans them up.
ALTER TABLE AttributeDefinition DROP COLUMN allow_other;
