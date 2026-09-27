-- Phase 6: riders and deliveries.

CREATE TYPE vehicle_type AS ENUM ('BICYCLE', 'SCOOTER', 'MOTORCYCLE', 'EV_SCOOTER');

CREATE TABLE rider_profiles (
    user_id         UUID PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    vehicle_type    vehicle_type NOT NULL DEFAULT 'SCOOTER',
    -- Indian registration number, e.g. KA01AB1234 (not required for bicycles).
    vehicle_number  TEXT CHECK (vehicle_number IS NULL OR vehicle_number ~ '^[A-Z]{2}[0-9]{1,2}[A-Z]{0,3}[0-9]{1,4}$'),
    is_online       BOOLEAN NOT NULL DEFAULT FALSE,
    last_seen_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TRIGGER trg_rider_profiles_updated_at BEFORE UPDATE ON rider_profiles
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- An order can be reassigned (the rider drops it), so it may have several
-- delivery rows, but only one active. A rider has one active delivery at a time.
CREATE TABLE deliveries (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id             UUID NOT NULL REFERENCES orders (id) ON DELETE CASCADE,
    rider_id             UUID NOT NULL REFERENCES users (id),
    store_id             UUID NOT NULL REFERENCES dark_stores (id),
    assigned_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    picked_up_at         TIMESTAMPTZ,
    departed_at          TIMESTAMPTZ,
    delivered_at         TIMESTAMPTZ,
    -- Wrong OTP entries; the delivery locks after too many.
    otp_attempts         INTEGER NOT NULL DEFAULT 0,
    cod_collected_paise  BIGINT CHECK (cod_collected_paise IS NULL OR cod_collected_paise >= 0),
    -- Set when the rider drops the job or the order is cancelled.
    ended_at             TIMESTAMPTZ
);

CREATE UNIQUE INDEX uq_deliveries_one_active_per_order ON deliveries (order_id)
    WHERE delivered_at IS NULL AND ended_at IS NULL;
CREATE UNIQUE INDEX uq_deliveries_one_active_per_rider ON deliveries (rider_id)
    WHERE delivered_at IS NULL AND ended_at IS NULL;
CREATE INDEX idx_deliveries_rider_history ON deliveries (rider_id, assigned_at DESC);
