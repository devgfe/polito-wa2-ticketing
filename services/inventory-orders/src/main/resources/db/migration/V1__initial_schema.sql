SET LOCAL lock_timeout = '2s';
SET LOCAL statement_timeout = '60s';

CREATE TABLE venue (
    id        BIGINT PRIMARY KEY,
    time_zone TEXT NOT NULL
);

CREATE TABLE sector (
    venue_id BIGINT NOT NULL REFERENCES venue (id),
    id       BIGINT NOT NULL,
    PRIMARY KEY (venue_id, id)
);

CREATE TABLE seat (
    venue_id  BIGINT NOT NULL,
    sector_id BIGINT NOT NULL,
    row       TEXT NOT NULL,
    number    INTEGER NOT NULL CHECK (number >= 1),
    PRIMARY KEY (venue_id, sector_id, row, number),
    FOREIGN KEY (venue_id, sector_id) REFERENCES sector (venue_id, id)
);

CREATE TABLE event (
    id           BIGINT PRIMARY KEY,
    venue_id     BIGINT NOT NULL REFERENCES venue (id),
    organizer_id UUID NOT NULL,
    sale_start   TIMESTAMP NOT NULL,
    currency     CHAR(3) NOT NULL CHECK (currency ~ '^[A-Z]{3}$'),
    UNIQUE (id, venue_id)
);

CREATE TABLE session (
    event_id  BIGINT NOT NULL REFERENCES event (id),
    id        BIGINT NOT NULL,
    starts_at TIMESTAMP NOT NULL,
    sale_end  TIMESTAMP NOT NULL,
    version   BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (event_id, id)
);

CREATE TABLE price_tier (
    event_id   BIGINT NOT NULL,
    session_id BIGINT NOT NULL,
    id         BIGINT NOT NULL,
    price      BIGINT NOT NULL CHECK (price >= 0),
    PRIMARY KEY (event_id, session_id, id),
    FOREIGN KEY (event_id, session_id) REFERENCES session (event_id, id)
);

CREATE TABLE session_seat (
    event_id      BIGINT NOT NULL,
    session_id    BIGINT NOT NULL,
    venue_id      BIGINT NOT NULL,
    sector_id     BIGINT NOT NULL,
    row           TEXT NOT NULL,
    number        INTEGER NOT NULL,
    price_tier_id BIGINT NOT NULL,
    state         TEXT NOT NULL CHECK (state IN ('FREE', 'HELD', 'SOLD')),
    customer_id   UUID,
    expires_at    TIMESTAMPTZ,
    hold_token    UUID,
    PRIMARY KEY (event_id, session_id, venue_id, sector_id, row, number),
    FOREIGN KEY (event_id, venue_id) REFERENCES event (id, venue_id),
    FOREIGN KEY (event_id, session_id) REFERENCES session (event_id, id),
    FOREIGN KEY (venue_id, sector_id, row, number) REFERENCES seat (venue_id, sector_id, row, number),
    FOREIGN KEY (event_id, session_id, price_tier_id) REFERENCES price_tier (event_id, session_id, id),
    CHECK ((state = 'FREE' AND customer_id IS NULL AND expires_at IS NULL AND hold_token IS NULL)
        OR (state = 'HELD' AND customer_id IS NOT NULL AND expires_at IS NOT NULL AND hold_token IS NOT NULL)
        OR (state = 'SOLD' AND expires_at IS NULL AND hold_token IS NULL))
);

CREATE TABLE orders (
    id                  BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    event_id            BIGINT NOT NULL,
    session_id          BIGINT NOT NULL,
    customer_id         UUID NOT NULL,
    customer_email      TEXT NOT NULL,
    currency            CHAR(3) NOT NULL CHECK (currency ~ '^[A-Z]{3}$'),
    ordered_at          TIMESTAMPTZ NOT NULL,
    status              TEXT NOT NULL CHECK (status IN ('PENDING', 'PAID', 'FAILED', 'CANCELLED')),
    payer_action_url    TEXT,
    paid_at             TIMESTAMPTZ,
    failed_at           TIMESTAMPTZ,
    failure_reason      TEXT CHECK (failure_reason IN ('HOLD_EXPIRED', 'PAYMENT_DECLINED', 'PAYMENT_EXPIRED')),
    cancelled_at        TIMESTAMPTZ,
    cancellation_reason TEXT,
    CHECK ((status IN ('PAID', 'CANCELLED')) = (paid_at IS NOT NULL)),
    CHECK (status = 'PENDING' OR payer_action_url IS NULL),
    CHECK (status <> 'FAILED' OR (failed_at IS NOT NULL AND failure_reason IS NOT NULL)),
    CHECK (status = 'FAILED' OR (failed_at IS NULL AND failure_reason IS NULL)),
    CHECK (status <> 'CANCELLED' OR (cancelled_at IS NOT NULL AND cancellation_reason IS NOT NULL)),
    CHECK (status = 'CANCELLED' OR (cancelled_at IS NULL AND cancellation_reason IS NULL)),
    UNIQUE (id, event_id, session_id),
    FOREIGN KEY (event_id, session_id) REFERENCES session (event_id, id)
);

CREATE INDEX orders_by_customer ON orders (customer_id, ordered_at, id);

CREATE TABLE order_item (
    order_id          BIGINT NOT NULL,
    event_id          BIGINT NOT NULL,
    session_id        BIGINT NOT NULL,
    venue_id          BIGINT NOT NULL,
    sector_id         BIGINT NOT NULL,
    row               TEXT NOT NULL,
    number            INTEGER NOT NULL,
    holder_first_name TEXT NOT NULL,
    holder_last_name  TEXT NOT NULL,
    price             BIGINT NOT NULL CHECK (price >= 0),
    PRIMARY KEY (order_id, event_id, session_id, venue_id, sector_id, row, number),
    FOREIGN KEY (order_id, event_id, session_id) REFERENCES orders (id, event_id, session_id),
    FOREIGN KEY (event_id, session_id, venue_id, sector_id, row, number)
        REFERENCES session_seat (event_id, session_id, venue_id, sector_id, row, number)
);

CREATE TABLE idempotency_key (
    customer_id  UUID NOT NULL,
    key          UUID NOT NULL,
    order_id     BIGINT NOT NULL UNIQUE REFERENCES orders (id),
    request_hash TEXT NOT NULL,
    expires_at   TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (customer_id, key)
);

CREATE INDEX idempotency_key_expiring ON idempotency_key (expires_at);