-- Drop the FK from UserAuthProvider to Member so that
-- auth provider records can be created before a Member record exists.
-- This allows new Auth0 signups to authenticate before completing the signup form.
ALTER TABLE UserAuthProvider DROP CONSTRAINT userauthprovider_user_id_fkey;
