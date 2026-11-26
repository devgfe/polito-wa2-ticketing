package it.polito.wa2.inventoryorders.seats.service

import it.polito.wa2.inventoryorders.common.CustomerId
import it.polito.wa2.inventoryorders.common.SessionSeatId
import it.polito.wa2.inventoryorders.config.SeatsProperties
import it.polito.wa2.inventoryorders.errors.HoldChanged
import it.polito.wa2.inventoryorders.errors.HoldLimitReached
import it.polito.wa2.inventoryorders.errors.NotFound
import it.polito.wa2.inventoryorders.errors.SalesClosed
import it.polito.wa2.inventoryorders.errors.SeatTaken
import it.polito.wa2.inventoryorders.seats.domain.SessionSeat
import org.springframework.stereotype.Service
import org.springframework.transaction.annotation.Transactional
import java.time.Clock
import java.util.UUID

/** Outcome of placeHold: the held seat, and whether the hold was created now (201) or already existed (200). */
data class PlaceHoldResult(val seat: SessionSeat.Held, val created: Boolean)

/** The use cases of the holds: place, read, release, list. */
@Service
class HoldService(
    private val seats: SessionSeatRepository,
    private val clock: Clock,
    private val props: SeatsProperties,
) {

    @Transactional
    fun placeHold(id: SessionSeatId, customer: CustomerId): PlaceHoldResult {
        val now = clock.instant()

        // 1. The session exists and its sales are open
        val onSale = seats.isOnSale(id.session, now) ?: throw NotFound("${id.session}")
        if (!onSale) throw SalesClosed(id.session)

        // 2. Limit per customer; the requested seat is excluded, so a repeated POST is not refused
        val limit = props.maxHoldsPerSession
        if (seats.countActiveHolds(customer, id.session, id.seat, now) >= limit) {
            throw HoldLimitReached(limit)
        }

        // 3. The conditional UPDATE decides who gets the seat (ADR-0003)
        val held = seats.tryHold(id, customer, props.holdTtl)
        if (held != null) return PlaceHoldResult(held, created = true)

        // 4. No record changed: read the seat to know why (ADR-0003)
        val current = seats.find(id) ?: throw NotFound("seat ${id.seat} of ${id.session}")
        val mine = current.heldBy(customer, now)
        if (mine != null) return PlaceHoldResult(mine, created = false)   // already mine: 200, expiry unchanged
        throw SeatTaken(id.seat)
    }

    @Transactional(readOnly = true)
    fun getHold(id: SessionSeatId, customer: CustomerId): SessionSeat.Held =
        seats.find(id)?.heldBy(customer, clock.instant())
            ?: throw NotFound("hold of seat ${id.seat} of ${id.session}")

    @Transactional
    fun releaseHold(id: SessionSeatId, customer: CustomerId, token: UUID) {
        if (seats.release(id, customer, token)) return    // released now: 204

        val current = seats.find(id) ?: throw NotFound("seat ${id.seat} of ${id.session}")
        if (current.isHeldAt(clock.instant())) throw HoldChanged(id.seat)    // a newer hold: 412
        // FREE, SOLD or expired: no longer held, 204 (ADR-0006)
    }

    @Transactional(readOnly = true)
    fun listHolds(customer: CustomerId): List<SessionSeat.Held> =
        seats.activeHolds(customer, clock.instant())
}