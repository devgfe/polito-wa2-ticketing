SET LOCAL lock_timeout = '2s';
SET LOCAL statement_timeout = '60s';

CREATE TABLE venue (
    id        BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    name      TEXT NOT NULL,
    city      TEXT NOT NULL,
    time_zone TEXT NOT NULL,
    owner_id  UUID NOT NULL
);

CREATE TABLE sector (
    venue_id BIGINT NOT NULL REFERENCES venue (id),
    id       BIGINT GENERATED ALWAYS AS IDENTITY,
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
    id           BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    venue_id     BIGINT NOT NULL REFERENCES venue (id),
    organizer_id UUID NOT NULL,
    name         TEXT NOT NULL,
    sale_start   TIMESTAMPTZ NOT NULL,
    published_at TIMESTAMPTZ,
    currency     CHAR(3) NOT NULL CHECK (currency ~ '^[A-Z]{3}$')
);

CREATE TABLE session (
    event_id  BIGINT NOT NULL REFERENCES event (id),
    id        BIGINT GENERATED ALWAYS AS IDENTITY,
    starts_at TIMESTAMPTZ NOT NULL,
    ends_at   TIMESTAMPTZ NOT NULL,
    sale_end  TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (event_id, id),
    CHECK (ends_at > starts_at)
);

CREATE TABLE price_tier (
    event_id   BIGINT NOT NULL,
    session_id BIGINT NOT NULL,
    id         BIGINT GENERATED ALWAYS AS IDENTITY,
    name       TEXT NOT NULL,
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
    PRIMARY KEY (event_id, session_id, venue_id, sector_id, row, number),
    FOREIGN KEY (event_id, session_id) REFERENCES session (event_id, id),
    FOREIGN KEY (venue_id, sector_id, row, number) REFERENCES seat (venue_id, sector_id, row, number),
    FOREIGN KEY (event_id, session_id, price_tier_id) REFERENCES price_tier (event_id, session_id, id)
);