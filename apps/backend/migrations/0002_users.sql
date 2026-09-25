-- Users and roles. Postgres is the source of truth for roles; Firebase only
-- proves who the caller is.

CREATE TYPE user_role AS ENUM ('CUSTOMER', 'PICKER', 'RIDER', 'ADMIN');

CREATE TABLE users (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    firebase_uid TEXT NOT NULL UNIQUE CHECK (length(firebase_uid) BETWEEN 1 AND 128),
    -- E.164; India-only (+91 followed by a 10-digit mobile number).
    phone        TEXT NOT NULL UNIQUE CHECK (phone ~ '^\+91[6-9][0-9]{9}$'),
    name         TEXT CHECK (name IS NULL OR length(name) BETWEEN 1 AND 100),
    role         user_role NOT NULL DEFAULT 'CUSTOMER',
    -- Dark store a PICKER/RIDER works at.
    -- TODO(phase-3): add FK to dark_stores(id) once that table exists.
    store_id     UUID,
    is_active    BOOLEAN NOT NULL DEFAULT TRUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_users_role ON users (role) WHERE role <> 'CUSTOMER';
CREATE INDEX idx_users_store ON users (store_id) WHERE store_id IS NOT NULL;

CREATE TRIGGER trg_users_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
