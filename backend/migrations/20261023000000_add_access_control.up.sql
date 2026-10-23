-- Access control ("kulunvalvonta") for meetings: members show a rotating QR
-- pass, door staff scan it in or out, and the log is kept for the minutes.

CREATE TYPE access_event_status AS ENUM ('not_started', 'running', 'paused', 'ended');
CREATE TYPE access_entry_direction AS ENUM ('in', 'out');
-- auto: written when the event ends while the member is still inside.
CREATE TYPE access_entry_method AS ENUM ('qr', 'manual', 'auto');

CREATE TABLE AccessEvent (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name          TEXT NOT NULL CHECK (name <> ''),
    event_date    DATE NOT NULL,
    -- A member gets in while holding any of these roles.
    allowed_roles TEXT[] NOT NULL CHECK (cardinality(allowed_roles) > 0),
    status        access_event_status NOT NULL DEFAULT 'not_started',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE AccessEntry (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id    UUID NOT NULL REFERENCES AccessEvent(id) ON DELETE CASCADE,
    user_id     UUID NOT NULL REFERENCES Member(user_id) ON DELETE CASCADE,
    direction   access_entry_direction NOT NULL,
    method      access_entry_method NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Who scanned or entered it; NULL for automatic entries.
    recorded_by UUID REFERENCES Member(user_id) ON DELETE SET NULL
);

CREATE INDEX AccessEntry_event_idx ON AccessEntry (event_id, recorded_at);

CREATE TABLE AccessStatusChange (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id   UUID NOT NULL REFERENCES AccessEvent(id) ON DELETE CASCADE,
    status     access_event_status NOT NULL,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    changed_by UUID REFERENCES Member(user_id) ON DELETE SET NULL
);

CREATE INDEX AccessStatusChange_event_idx ON AccessStatusChange (event_id, changed_at);

-- Who was inside at a moment, e.g. for counting the votes of an election.
CREATE TABLE AccessSnapshot (
    id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES AccessEvent(id) ON DELETE CASCADE,
    label    TEXT CHECK (label <> ''),
    taken_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    taken_by UUID REFERENCES Member(user_id) ON DELETE SET NULL,
    user_ids UUID[] NOT NULL
);

CREATE INDEX AccessSnapshot_event_idx ON AccessSnapshot (event_id, taken_at);
