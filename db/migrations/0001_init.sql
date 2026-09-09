-- vigo 0001_init
-- Marketplace core: shops list items from a master catalog; customers browse
-- nearby open shops. Orders, riders, carts and payouts come in later migrations.

CREATE EXTENSION IF NOT EXISTS postgis;

CREATE OR REPLACE FUNCTION set_updated_at() RETURNS trigger AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ---------------------------------------------------------------- catalog --
-- Master catalog. Shops do NOT invent items; they list products from here with
-- their own price and stock. Without this, 50 shops produce "Amul Milk 500ml"
-- nine different ways and search falls apart.

CREATE TABLE categories (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name       TEXT NOT NULL UNIQUE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE products (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    category_id UUID NOT NULL REFERENCES categories(id),
    name        TEXT NOT NULL,
    -- '' rather than NULL so the UNIQUE below actually catches duplicates
    brand       TEXT NOT NULL DEFAULT '',
    unit        TEXT NOT NULL,              -- '500 ml', '1 kg', '6 pcs'
    image_url   TEXT,
    is_active   BOOLEAN NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (name, brand, unit)
);

CREATE INDEX products_category_idx ON products (category_id) WHERE is_active;

-- ------------------------------------------------------------------ people --
-- Separate tables per role, not one `users` table with a role column: three
-- separate apps (Decision 4), three different sets of attributes.

CREATE TABLE shop_owners (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    phone      TEXT NOT NULL UNIQUE CHECK (phone ~ '^\+91[6-9][0-9]{9}$'),
    name       TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE customers (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    phone      TEXT NOT NULL UNIQUE CHECK (phone ~ '^\+91[6-9][0-9]{9}$'),
    name       TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ------------------------------------------------------------------- shops --

CREATE TABLE shops (
    id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL REFERENCES shop_owners(id),
    name     TEXT NOT NULL,
    address  TEXT NOT NULL,

    -- ST_MakePoint(longitude, latitude) - lng FIRST
    location geography(Point, 4326) NOT NULL,

    status   TEXT NOT NULL DEFAULT 'pending'
             CHECK (status IN ('pending', 'active', 'suspended')),

    -- is_open: shutters up, serving walk-ins.
    -- is_accepting_orders: willing to take app orders right now.
    -- Deliberately separate - a shop can be open but too swamped for app orders.
    is_open             BOOLEAN NOT NULL DEFAULT FALSE,
    is_accepting_orders BOOLEAN NOT NULL DEFAULT TRUE,

    opens_at  TIME NOT NULL DEFAULT '08:00',
    closes_at TIME NOT NULL DEFAULT '22:00',

    -- 10-20 min promise: radius is a PRODUCT decision, not config.
    delivery_radius_m INTEGER NOT NULL DEFAULT 2500
                      CHECK (delivery_radius_m BETWEEN 500 AND 5000),

    -- SLA, measured not declared. Drives ranking in customer search.
    avg_prep_seconds INTEGER NOT NULL DEFAULT 300 CHECK (avg_prep_seconds > 0),
    acceptance_rate  NUMERIC(4,3) NOT NULL DEFAULT 1.000
                     CHECK (acceptance_rate BETWEEN 0 AND 1),

    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- The index that makes "shops near me" an index lookup instead of a full scan.
CREATE INDEX shops_location_gix ON shops USING GIST (location);
CREATE INDEX shops_owner_idx    ON shops (owner_id);

-- ------------------------------------------------------------- shop_items --
-- A shop's listing of a master product: their price, their stock.

CREATE TABLE shop_items (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    shop_id    UUID NOT NULL REFERENCES shops(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES products(id),

    -- Money is integer paise. Never floats. (Scope: INR only.)
    price_paise INTEGER NOT NULL CHECK (price_paise > 0),

    stock_qty    INTEGER NOT NULL DEFAULT 0 CHECK (stock_qty >= 0),
    is_available BOOLEAN NOT NULL DEFAULT TRUE,

    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (shop_id, product_id)
);

CREATE INDEX shop_items_product_idx ON shop_items (product_id);
CREATE INDEX shop_items_sellable_idx ON shop_items (shop_id)
    WHERE is_available AND stock_qty > 0;

-- ---------------------------------------------------------------- triggers --

CREATE TRIGGER products_updated_at    BEFORE UPDATE ON products
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER shop_owners_updated_at BEFORE UPDATE ON shop_owners
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER customers_updated_at   BEFORE UPDATE ON customers
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER shops_updated_at       BEFORE UPDATE ON shops
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER shop_items_updated_at  BEFORE UPDATE ON shop_items
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
