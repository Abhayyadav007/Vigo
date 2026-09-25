-- Phase 4: addresses, carts, orders, order state log, payments.

CREATE TABLE addresses (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    -- "Home", "Work", "Other"...
    label       TEXT NOT NULL CHECK (length(label) BETWEEN 1 AND 30),
    -- House / flat / floor.
    line1       TEXT NOT NULL CHECK (length(line1) BETWEEN 1 AND 200),
    -- Building, street, area.
    line2       TEXT CHECK (line2 IS NULL OR length(line2) <= 200),
    landmark    TEXT CHECK (landmark IS NULL OR length(landmark) <= 100),
    city        TEXT NOT NULL CHECK (length(city) BETWEEN 1 AND 80),
    -- Indian PIN code.
    pincode     TEXT NOT NULL CHECK (pincode ~ '^[1-9][0-9]{5}$'),
    location    GEOGRAPHY(Point, 4326) NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_addresses_user ON addresses (user_id);

CREATE TRIGGER trg_addresses_updated_at BEFORE UPDATE ON addresses
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- One cart per customer per store (catalog and prices are store-specific).
CREATE TABLE carts (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    store_id    UUID NOT NULL REFERENCES dark_stores (id) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, store_id)
);

CREATE TRIGGER trg_carts_updated_at BEFORE UPDATE ON carts
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE cart_items (
    cart_id     UUID NOT NULL REFERENCES carts (id) ON DELETE CASCADE,
    product_id  UUID NOT NULL REFERENCES products (id) ON DELETE CASCADE,
    quantity    INTEGER NOT NULL CHECK (quantity BETWEEN 1 AND 10),
    added_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (cart_id, product_id)
);

CREATE TYPE order_status AS ENUM (
    'PLACED', 'CONFIRMED', 'PICKING', 'PACKED', 'RIDER_ASSIGNED',
    'PICKED_UP', 'OUT_FOR_DELIVERY', 'DELIVERED', 'CANCELLED', 'PARTIALLY_FULFILLED'
);
CREATE TYPE payment_method AS ENUM ('COD', 'ONLINE');
CREATE TYPE payment_status AS ENUM ('PENDING', 'PAID', 'FAILED', 'REFUNDED');

-- Human-friendly order numbers: VG100001, VG100002, ...
CREATE SEQUENCE order_number_seq START 100001;

CREATE TABLE orders (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    number              BIGINT NOT NULL UNIQUE DEFAULT nextval('order_number_seq'),
    user_id             UUID NOT NULL REFERENCES users (id),
    store_id            UUID NOT NULL REFERENCES dark_stores (id),
    status              order_status NOT NULL DEFAULT 'PLACED',
    payment_method      payment_method NOT NULL,
    payment_status      payment_status NOT NULL DEFAULT 'PENDING',
    item_total_paise    BIGINT NOT NULL CHECK (item_total_paise > 0),
    mrp_total_paise     BIGINT NOT NULL CHECK (mrp_total_paise >= item_total_paise),
    delivery_fee_paise  BIGINT NOT NULL CHECK (delivery_fee_paise >= 0),
    total_paise         BIGINT NOT NULL CHECK (total_paise = item_total_paise + delivery_fee_paise),
    -- Snapshot: later edits to the saved address must not change past orders.
    address             JSONB NOT NULL,
    delivery_location   GEOGRAPHY(Point, 4326) NOT NULL,
    -- Shown to the customer; the rider enters it to complete delivery (phase 6).
    delivery_otp        TEXT NOT NULL CHECK (delivery_otp ~ '^[0-9]{4}$'),
    -- Client-supplied Idempotency-Key for POST /checkout.
    idempotency_key     TEXT NOT NULL CHECK (length(idempotency_key) BETWEEN 8 AND 64),
    -- Payment gateway's order id (Razorpay), for webhooks.
    gateway_order_id    TEXT UNIQUE,
    cancel_reason       TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, idempotency_key)
);

CREATE INDEX idx_orders_user ON orders (user_id, created_at DESC);
CREATE INDEX idx_orders_store_active ON orders (store_id, created_at)
    WHERE status NOT IN ('DELIVERED', 'CANCELLED', 'PARTIALLY_FULFILLED');

CREATE TRIGGER trg_orders_updated_at BEFORE UPDATE ON orders
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- Price/name snapshot per line.
CREATE TABLE order_items (
    order_id          UUID NOT NULL REFERENCES orders (id) ON DELETE CASCADE,
    product_id        UUID NOT NULL REFERENCES products (id),
    name              TEXT NOT NULL,
    brand             TEXT,
    unit_label        TEXT NOT NULL,
    image_url         TEXT,
    quantity          INTEGER NOT NULL CHECK (quantity BETWEEN 1 AND 10),
    unit_price_paise  BIGINT NOT NULL CHECK (unit_price_paise > 0),
    unit_mrp_paise    BIGINT NOT NULL CHECK (unit_mrp_paise >= unit_price_paise),
    -- Set by the picker (phase 5); NULL until picked.
    picked_quantity   INTEGER CHECK (picked_quantity BETWEEN 0 AND quantity),
    PRIMARY KEY (order_id, product_id)
);

-- Audit log: one row per status change.
CREATE TABLE order_status_events (
    id             BIGSERIAL PRIMARY KEY,
    order_id       UUID NOT NULL REFERENCES orders (id) ON DELETE CASCADE,
    from_status    order_status,
    to_status      order_status NOT NULL,
    actor_user_id  UUID REFERENCES users (id),
    note           TEXT,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_order_status_events_order ON order_status_events (order_id, id);

-- Webhook deliveries, deduplicated by the provider's event id.
CREATE TABLE payment_events (
    id           BIGSERIAL PRIMARY KEY,
    provider     TEXT NOT NULL,
    event_id     TEXT NOT NULL,
    event_type   TEXT NOT NULL,
    order_id     UUID REFERENCES orders (id),
    payload      JSONB NOT NULL,
    received_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (provider, event_id)
);
