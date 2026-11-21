SET LOCAL lock_timeout = '2s';
SET LOCAL statement_timeout = '60s';

CREATE TABLE psp_operation (
    id             BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    kind           TEXT NOT NULL CHECK (kind IN ('PAYMENT', 'REFUND')),
    order_id       BIGINT NOT NULL,
    status         TEXT NOT NULL,
    amount         BIGINT NOT NULL CHECK (amount > 0),
    currency       CHAR(3) NOT NULL CHECK (currency ~ '^[A-Z]{3}$'),
    created_at     TIMESTAMPTZ NOT NULL,
    failure_reason TEXT,
    CHECK ((kind = 'PAYMENT' AND status IN ('INITIATED', 'CREATED', 'AUTHORIZED', 'PROCESSING', 'CAPTURED', 'FAILED'))
        OR (kind = 'REFUND'  AND status IN ('INITIATED', 'PENDING', 'COMPLETED', 'FAILED'))),
    CHECK (failure_reason IS NULL OR status = 'FAILED')
);

CREATE TABLE paypal_payment (
    operation_id      BIGINT PRIMARY KEY REFERENCES psp_operation (id),
    expires_at        TIMESTAMPTZ NOT NULL,
    paypal_order_id   TEXT UNIQUE,
    payer_action_url  TEXT,
    paypal_capture_id TEXT UNIQUE
);

CREATE TABLE paypal_refund (
    operation_id     BIGINT PRIMARY KEY REFERENCES psp_operation (id),
    paypal_refund_id TEXT UNIQUE
);

CREATE UNIQUE INDEX one_active_payment_per_order
    ON psp_operation (order_id) WHERE kind = 'PAYMENT' AND status <> 'FAILED';

CREATE UNIQUE INDEX one_refund_per_order
    ON psp_operation (order_id) WHERE kind = 'REFUND' AND status <> 'FAILED';