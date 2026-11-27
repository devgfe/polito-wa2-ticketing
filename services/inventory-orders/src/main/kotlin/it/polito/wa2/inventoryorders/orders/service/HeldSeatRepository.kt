package it.polito.wa2.inventoryorders.orders.service

import it.polito.wa2.inventoryorders.common.CustomerId
import it.polito.wa2.inventoryorders.common.SeatRef
import it.polito.wa2.inventoryorders.common.SessionRef
import it.polito.wa2.inventoryorders.orders.domain.Money
import java.time.Instant

/** Data of a held seat that the order needs: its price. */
data class HeldSeat(val price: Money)

/** Port on session_seat for the orders. */
interface HeldSeatRepository {

    /** The seat, locked until commit, if the customer holds it at `now`; null otherwise. */
    fun lockHeldSeat(session: SessionRef, seat: SeatRef, customer: CustomerId, now: Instant): HeldSeat?
}