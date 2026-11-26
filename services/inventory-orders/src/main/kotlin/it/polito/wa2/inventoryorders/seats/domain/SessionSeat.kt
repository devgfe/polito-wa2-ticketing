package it.polito.wa2.inventoryorders.seats.domain

import it.polito.wa2.inventoryorders.common.CustomerId
import it.polito.wa2.inventoryorders.common.SessionSeatId
import java.time.Instant
import java.util.UUID

/** The data of a hold. */
data class Hold(
    val customer: CustomerId,
    val token: UUID,
    val expiresAt: Instant,
) {
    /** An expired hold is treated as free, even before anyone releases it (ADR-0003). */
    fun isActiveAt(now: Instant): Boolean = expiresAt.isAfter(now)
}

/** One record of session_seat: one seat in one session, in one of its three states (ADR-0003). */
sealed interface SessionSeat {
    val id: SessionSeatId

    data class Free(override val id: SessionSeatId) : SessionSeat

    data class Sold(override val id: SessionSeatId) : SessionSeat

    data class Held(override val id: SessionSeatId, val hold: Hold) : SessionSeat

    /** Held by anyone, and not expired. */
    fun isHeldAt(now: Instant): Boolean =
        this is Held && hold.isActiveAt(now)

    /** This seat, if this customer holds it now; null otherwise. */
    fun heldBy(customer: CustomerId, now: Instant): Held? =
        if (this is Held && hold.customer == customer && hold.isActiveAt(now)) this else null
}