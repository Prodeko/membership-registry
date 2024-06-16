CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE TABLE Member (
    -- id, email comes from identity service
    user_id uuid primary key,
    first_name text not null,
    last_name text not null,
    home_municipality text not null,
    has_accepted_policies boolean not null,
    full_name TEXT GENERATED ALWAYS AS (first_name || ' ' || last_name) STORED
);

CREATE TABLE Role (
    name text primary key
);

CREATE TABLE Application (
    user_id uuid not null,
    role_name text not null,
    timestamp timestamptz not null,
    stripe_payment_id text,
    application_text text,
    primary key (role_name, user_id),
    foreign key (role_name) references Role(name)
);

CREATE TABLE RoleMember (
    user_id uuid not null,
    role_name text not null,
    valid_from date not null,
    valid_until date,
    primary key (user_id, role_name, valid_from),
    foreign key (user_id) references Member(user_id),
    foreign key (role_name) references Role(name)
);

CREATE INDEX idx_name_trgm_gin ON Member USING gin (full_name gin_trgm_ops);