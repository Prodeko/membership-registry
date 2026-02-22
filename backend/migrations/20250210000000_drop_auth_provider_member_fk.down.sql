ALTER TABLE UserAuthProvider
ADD CONSTRAINT userauthprovider_user_id_fkey
FOREIGN KEY (user_id) REFERENCES Member(user_id) ON DELETE CASCADE;
