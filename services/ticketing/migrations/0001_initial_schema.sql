SET LOCAL lock_timeout = '2s';
SET LOCAL statement_timeout = '60s';

CREATE TABLE venue (
    id        BIGINT PRIMARY KEY,
    name      TEXT NOT NULL,
    city      TEXT NOT NULL,
    time_zone TEXT NOT NULL
);

CREATE TABLE sector (
    venue_id BIGINT NOT NULL REFERENCES venue (id),
    id       BIGINT NOT NULL,
    name     TEXT NOT NULL,
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
    id       BIGINT PRIMARY KEY,
    venue_id BIGINT NOT NULL REFERENCES venue (id),
    name     TEXT NOT NULL,
    UNIQUE (id, venue_id)
);

CREATE TABLE session (
    event_id  BIGINT NOT NULL REFERENCES event (id),
    id        BIGINT NOT NULL,
    starts_at TIMESTAMP NOT NULL,
    ends_at   TIMESTAMP NOT NULL,
    PRIMARY KEY (event_id, id),
    CHECK (ends_at > starts_at)
);

CREATE TABLE session_seat (
    event_id   BIGINT NOT NULL,
    session_id BIGINT NOT NULL,
    venue_id   BIGINT NOT NULL,
    sector_id  BIGINT NOT NULL,
    row        TEXT NOT NULL,
    number     INTEGER NOT NULL,
    PRIMARY KEY (event_id, session_id, venue_id, sector_id, row, number),
    FOREIGN KEY (event_id, venue_id) REFERENCES event (id, venue_id),
    FOREIGN KEY (event_id, session_id) REFERENCES session (event_id, id),
    FOREIGN KEY (venue_id, sector_id, row, number) REFERENCES seat (venue_id, sector_id, row, number)
);

CREATE TABLE orders (
    id             BIGINT PRIMARY KEY,
    event_id       BIGINT NOT NULL,
    session_id     BIGINT NOT NULL,
    customer_id    UUID NOT NULL,
    customer_email TEXT NOT NULL,
    currency       CHAR(3) NOT NULL CHECK (currency ~ '^[A-Z]{3}$'),
    status         TEXT NOT NULL CHECK (status IN ('PAID', 'CANCELLED')),
    UNIQUE (id, event_id, session_id),
    FOREIGN KEY (event_id, session_id) REFERENCES session (event_id, id)
);

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

CREATE TABLE ticket (
    id            UUID PRIMARY KEY,
    order_id      BIGINT NOT NULL,
    event_id      BIGINT NOT NULL,
    session_id    BIGINT NOT NULL,
    venue_id      BIGINT NOT NULL,
    sector_id     BIGINT NOT NULL,
    row           TEXT NOT NULL,
    number        INTEGER NOT NULL,
    pdf_key       TEXT,
    gate_staff_id UUID,
    scanned_at    TIMESTAMPTZ,
    UNIQUE (order_id, event_id, session_id, venue_id, sector_id, row, number),
    FOREIGN KEY (order_id, event_id, session_id, venue_id, sector_id, row, number)
        REFERENCES order_item (order_id, event_id, session_id, venue_id, sector_id, row, number),
    CHECK ((gate_staff_id IS NULL) = (scanned_at IS NULL))
);