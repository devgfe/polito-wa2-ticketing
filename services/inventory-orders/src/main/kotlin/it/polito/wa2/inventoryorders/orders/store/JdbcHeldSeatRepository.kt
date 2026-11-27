package it.polito.wa2.inventoryorders.orders.store

import it.polito.wa2.inventoryorders.common.CustomerId
import it.polito.wa2.inventoryorders.common.SeatRef
import it.polito.wa2.inventoryorders.common.SessionRef
import it.polito.wa2.inventoryorders.common.SessionSeatId
import it.polito.wa2.inventoryorders.orders.domain.Money
import it.polito.wa2.inventoryorders.orders.service.HeldSeat
import it.polito.wa2.inventoryorders.orders.service.HeldSeatRepository
import org.springframework.jdbc.core.simple.JdbcClient
import org.springframework.stereotype.Repository
import java.time.Instant
import java.time.ZoneOffset

/**
 * Adapter of HeldSeatRepository (the orders' port on session_seat) on PostgreSQL.
 * The venue of the seat key comes from the event, as in ADR-0003.
 */
@Repository
class JdbcHeldSeatRepository(private val jdbc: JdbcClient) : HeldSeatRepository {

    /**
     * FOR UPDATE locks the row until the commit of createOrder, so the seat cannot change state
     * underneath the order; the price and the currency come from the copies of the catalogue.
     */
    override fun lockHeldSeat(session: SessionRef, seat: SeatRef, customer: CustomerId, now: Instant): HeldSeat? =
        jdbc.sql(
            """
            SELECT p.price, e.currency
            FROM session_seat ss
            JOIN event e        ON e.id = ss.event_id AND e.venue_id = ss.venue_id
            JOIN price_tier p   ON p.event_id = ss.event_id AND p.session_id = ss.session_id AND p.id = ss.price_tier_id
            WHERE ss.event_id = :eventId AND ss.session_id = :sessionId
              AND ss.venue_id = (SELECT venue_id FROM event WHERE id = :eventId)
              AND ss.sector_id = :sectorId AND ss.row = :row AND ss.number = :number
              AND ss.state = 'HELD' AND ss.customer_id = :customer AND ss.expires_at > :now
            FOR UPDATE
            """
        )
            .seatParams(SessionSeatId(session, seat))
            .param("customer", customer.value)
            .param("now", now.atOffset(ZoneOffset.UTC))
            .query { rs, _ -> HeldSeat(Money(rs.getLong("price"), rs.getString("currency"))) }
            .optional().orElse(null)

    /** Binds the parameters of the seat key, as in JdbcSessionSeatRepository. */
    private fun JdbcClient.StatementSpec.seatParams(id: SessionSeatId): JdbcClient.StatementSpec =
        param("eventId", id.session.eventId)
            .param("sessionId", id.session.sessionId)
            .param("sectorId", id.seat.sectorId)
            .param("row", id.seat.row)
            .param("number", id.seat.number)
}
