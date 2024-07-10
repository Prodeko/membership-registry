-- Add down migration script here

DROP TABLE IF EXISTS RoleMember;

DROP TABLE IF EXISTS Application;

DROP TABLE IF EXISTS ApplicationTargetableRole;

DROP TABLE IF EXISTS Member;

DROP TABLE IF EXISTS Role;

DROP EXTENSION IF EXISTS pg_trgm;