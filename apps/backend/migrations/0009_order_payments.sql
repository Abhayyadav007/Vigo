-- Captured gateway payments, so cancellations can be refunded through the
-- provider. One per order: the gateway order (orders.gateway_order_id) takes
-- a single successful payment.
CREATE TABLE order_payments (
    order_id       UUID PRIMARY KEY REFERENCES orders (id),
    provider       TEXT NOT NULL,
    payment_id     TEXT NOT NULL UNIQUE,
    amount_paise   BIGINT NOT NULL CHECK (amount_paise > 0),
    captured_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Set once the provider accepted a refund request.
    refund_id      TEXT UNIQUE,
    refunded_at    TIMESTAMPTZ
);
