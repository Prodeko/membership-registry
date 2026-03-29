UPDATE Member SET home_municipality = '' WHERE home_municipality IS NULL;
ALTER TABLE Member ALTER COLUMN home_municipality SET NOT NULL;
