-- Phase 5: picking and packing at the dark store.

ALTER TABLE orders
    ADD COLUMN picker_id          UUID REFERENCES users (id),
    ADD COLUMN picking_started_at TIMESTAMPTZ,
    ADD COLUMN packed_at          TIMESTAMPTZ,
    -- How many bags the rider collects, and where they're staged (e.g. "S-03").
    ADD COLUMN bag_count          INTEGER CHECK (bag_count BETWEEN 1 AND 20),
    ADD COLUMN staging_slot       TEXT CHECK (staging_slot IS NULL OR length(staging_slot) BETWEEN 1 AND 16);

CREATE INDEX idx_orders_picker ON orders (picker_id) WHERE status = 'PICKING';
