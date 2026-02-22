ALTER TABLE Application
    ALTER COLUMN status DROP NOT NULL,
    ALTER COLUMN status TYPE text USING status::text;

DROP TYPE application_status;
