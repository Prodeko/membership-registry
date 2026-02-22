CREATE TYPE application_status AS ENUM ('unpaid', 'pending', 'approved', 'rejected');

ALTER TABLE Application
    ALTER COLUMN status TYPE application_status USING status::application_status,
    ALTER COLUMN status SET NOT NULL;
