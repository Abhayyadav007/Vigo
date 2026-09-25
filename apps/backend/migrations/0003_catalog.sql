-- Phase 3: dark stores (with PostGIS service areas), catalog, per-store inventory.
-- Money is always BIGINT paise.

CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE TABLE dark_stores (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Short human code, e.g. BLR-IND-01; shown on staff screens and labels.
    code          TEXT NOT NULL UNIQUE CHECK (code ~ '^[A-Z0-9-]{2,32}$'),
    name          TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 100),
    address       TEXT NOT NULL CHECK (length(address) BETWEEN 1 AND 300),
    location      GEOGRAPHY(Point, 4326) NOT NULL,
    service_area  GEOGRAPHY(Polygon, 4326) NOT NULL,
    is_active     BOOLEAN NOT NULL DEFAULT TRUE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_dark_stores_service_area ON dark_stores USING GIST (service_area);
CREATE INDEX idx_dark_stores_location ON dark_stores USING GIST (location);

CREATE TRIGGER trg_dark_stores_updated_at BEFORE UPDATE ON dark_stores
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- Staff belong to a store (TODO from 0002).
ALTER TABLE users
    ADD CONSTRAINT users_store_id_fkey
    FOREIGN KEY (store_id) REFERENCES dark_stores (id) ON DELETE SET NULL;

CREATE TABLE categories (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    parent_id   UUID REFERENCES categories (id) ON DELETE RESTRICT,
    name        TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 80),
    slug        TEXT NOT NULL UNIQUE CHECK (slug ~ '^[a-z0-9]+(-[a-z0-9]+)*$'),
    image_url   TEXT,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    is_active   BOOLEAN NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (parent_id IS DISTINCT FROM id)
);

CREATE INDEX idx_categories_parent ON categories (parent_id);

CREATE TRIGGER trg_categories_updated_at BEFORE UPDATE ON categories
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE products (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    category_id  UUID NOT NULL REFERENCES categories (id) ON DELETE RESTRICT,
    name         TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 150),
    slug         TEXT NOT NULL UNIQUE CHECK (slug ~ '^[a-z0-9]+(-[a-z0-9]+)*$'),
    brand        TEXT CHECK (brand IS NULL OR length(brand) BETWEEN 1 AND 80),
    description  TEXT CHECK (description IS NULL OR length(description) <= 2000),
    -- Pack size shown to shoppers, e.g. "500 g", "1 L", "6 pcs".
    unit_label   TEXT NOT NULL CHECK (length(unit_label) BETWEEN 1 AND 40),
    -- EAN/UPC scanned by pickers (phase 5).
    barcode      TEXT UNIQUE CHECK (barcode IS NULL OR barcode ~ '^[0-9]{8,14}$'),
    mrp_paise    BIGINT NOT NULL CHECK (mrp_paise > 0),
    price_paise  BIGINT NOT NULL CHECK (price_paise > 0),
    image_urls   TEXT[] NOT NULL DEFAULT '{}' CHECK (cardinality(image_urls) <= 8),
    is_active    BOOLEAN NOT NULL DEFAULT TRUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Selling above MRP is illegal in India (Legal Metrology Act).
    CHECK (price_paise <= mrp_paise)
);

CREATE INDEX idx_products_category ON products (category_id) WHERE is_active;
-- Substring / typo-tolerant search on name and brand.
CREATE INDEX idx_products_search ON products
    USING GIN ((name || ' ' || coalesce(brand, '')) gin_trgm_ops);

CREATE TRIGGER trg_products_updated_at BEFORE UPDATE ON products
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- Source of truth for stock. Redis mirrors it for reservations from phase 4.
CREATE TABLE store_inventory (
    store_id              UUID NOT NULL REFERENCES dark_stores (id) ON DELETE CASCADE,
    product_id            UUID NOT NULL REFERENCES products (id) ON DELETE CASCADE,
    quantity              INTEGER NOT NULL DEFAULT 0 CHECK (quantity >= 0),
    -- Shelf address for pickers, e.g. "A-03-2" (aisle-rack-shelf).
    bin_location          TEXT CHECK (bin_location IS NULL OR length(bin_location) BETWEEN 1 AND 32),
    -- Store-specific price; NULL means the product's base price.
    price_override_paise  BIGINT CHECK (price_override_paise IS NULL OR price_override_paise > 0),
    is_available          BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (store_id, product_id)
);

CREATE INDEX idx_store_inventory_product ON store_inventory (product_id);

CREATE TRIGGER trg_store_inventory_updated_at BEFORE UPDATE ON store_inventory
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
