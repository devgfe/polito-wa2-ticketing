package it.polito.wa2.inventoryorders.common

import java.util.UUID

/** Customer, as Keycloak identifies it. */
data class CustomerId(val value: UUID)

/** A session of an event: its key is the pair (event, session). */
data class SessionRef(val eventId: Long, val sessionId: Long) {
    init {
        require(eventId >= 1) { "eventId must be at least 1" }
        require(sessionId >= 1) { "sessionId must be at least 1" }
    }

    override fun toString() = "event $eventId session $sessionId"
}

/** Seat of the venue, as the API names it (SeatRef). */
data class SeatRef(val sectorId: Long, val row: String, val number: Int) {
    init {
        require(sectorId >= 1) { "sectorId must be at least 1" }
        require(row.length in 1..10) { "row must have 1 to 10 characters" }
        require(number >= 1) { "number must be at least 1" }
    }

    override fun toString() = "$sectorId/$row$number"
}

/**
 * Identifies one record of session_seat: a seat in a session.
 * The venue of its primary key is not here: it comes from the event (ADR-0003).
 */
data class SessionSeatId(val session: SessionRef, val seat: SeatRef)