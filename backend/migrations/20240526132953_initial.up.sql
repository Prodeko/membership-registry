CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE TABLE Member (
    -- id comes from identity service
    user_id uuid primary key,
    email text not null,
    first_name text not null,
    last_name text not null,
    home_municipality text not null,
    has_accepted_policies boolean not null,
    full_name TEXT GENERATED ALWAYS AS (first_name || ' ' || last_name) STORED
);

CREATE TABLE Role (
    name text primary key,
    description text,
    color text
);


CREATE TABLE RoleMember (
    user_id uuid not null,
    role_name text not null,
    valid_from date not null default current_date,
    valid_until date,
    primary key (user_id, role_name, valid_from),
    foreign key (user_id) references Member(user_id) ON DELETE CASCADE,
    foreign key (role_name) references Role(name)
);

-- This table is used to keep track of which roles can be applied to by users 
-- and to keep consitent valid_until dates for roles that are targetable.
CREATE TABLE ApplicationTargetableRole (
    role_name text,
    valid_until date,
    active boolean not null default true,
    payment_link text,
    optional_roles text[],
    primary key (role_name, valid_until),
    foreign key (role_name) references Role(name) ON DELETE CASCADE
);

CREATE TABLE Application (
    application_id uuid primary key default gen_random_uuid(),
    user_id uuid not null,
    role_name text not null,
    valid_until date not null,
    timestamp timestamptz not null,
    stripe_payment_id text,
    application_text text,
    status text not null,
    optional_roles text[],
    foreign key (role_name, valid_until) references ApplicationTargetableRole(role_name, valid_until)
);

CREATE TABLE SavedFilter (
    name text primary key,
    filtered_model text not null,
    owner_user_id uuid not null,
    visible_for_all boolean not null default false,
    search text null,
    sorting_col text null,
    sorting_desc boolean not null default false,
    custom_filters JSON null
);

CREATE INDEX idx_name_trgm_gin ON Member USING gin (full_name gin_trgm_ops);
CREATE INDEX idx_email_trgm_gin ON Member USING gin (email gin_trgm_ops);