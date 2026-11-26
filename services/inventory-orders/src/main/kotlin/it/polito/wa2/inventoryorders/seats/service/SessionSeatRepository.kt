package it.polito.wa2.inventoryorders.seats.service

import it.polito.wa2.inventoryorders.common.CustomerId
import it.polito.wa2.inventoryorders.common.SeatRef
import it.polito.wa2.inventoryorders.common.SessionRef
import it.polito.wa2.inventoryorders.common.SessionSeatId
import it.polito.wa2.inventoryorders.seats.domain.SessionSeat
import java.time.Instant
import java.time.Duration
import java.util.UUID

/**
 * Port on the table session_seat and on the read-only copies of the catalogue.
 */
interface SessionSeatRepository {

    /** Whether the sales of the session are open at `now`; null if the session does not exist. */
    fun isOnSale(session: SessionRef, now: Instant): Boolean?

    /** The seat in its current state; null if the session has no such seat. */
    fun find(id: SessionSeatId): SessionSeat?

    /** How many seats of the session the customer holds and are not expired at `now`, `except` excluded. */
    fun countActiveHolds(customer: CustomerId, session: SessionRef, except: SeatRef, now: Instant): Int

    /**
     * The conditional UPDATE of ADR-0003: holds the seat for `ttl` if it is free or its hold expired.
     * It uses the clock of the database: the arbiter of the race is the database, shared by all replicas.
     * The seat now held by the customer, or null if no record changed.
     */
    fun tryHold(id: SessionSeatId, customer: CustomerId, ttl: Duration): SessionSeat.Held?

    /** Frees the seat only if the customer holds it with that token (ADR-0006); true if a record changed. */
    fun release(id: SessionSeatId, customer: CustomerId, token: UUID): Boolean

    /** The seats held by the customer and not expired at `now`, in every session. */
    fun activeHolds(customer: CustomerId, now: Instant): List<SessionSeat.Held>
}