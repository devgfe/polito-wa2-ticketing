package it.polito.wa2.inventoryorders.seats.store

import it.polito.wa2.inventoryorders.common.CustomerId
import it.polito.wa2.inventoryorders.common.SeatRef
import it.polito.wa2.inventoryorders.common.SessionRef
import it.polito.wa2.inventoryorders.common.SessionSeatId
import it.polito.wa2.inventoryorders.seats.domain.Hold
import it.polito.wa2.inventoryorders.seats.domain.SeatMapSeat
import it.polito.wa2.inventoryorders.seats.domain.SeatStatus
import it.polito.wa2.inventoryorders.seats.domain.SessionSeat
import it.polito.wa2.inventoryorders.seats.service.SessionSeatRepository
import it.polito.wa2.inventoryorders.seats.domain.SeatStateChangedEvent
import org.springframework.context.ApplicationEventPublisher
import org.springframework.jdbc.core.simple.JdbcClient
import org.springframework.stereotype.Repository
import java.sql.ResultSet
import java.time.Duration
import java.time.Instant
import java.time.OffsetDateTime
import java.time.ZoneOffset
import java.util.UUID

/**
 * Full key of one record of session_seat. The venue is not in the API:
 * it comes from the event (ADR-0003), so the whole primary key index is used.
 */
private const val SEAT_KEY = """
    event_id = :eventId AND session_id = :sessionId
    AND venue_id = (SELECT venue_id FROM event WHERE id = :eventId)
    AND sector_id = :sectorId AND row = :row AND number = :number"""

/** Adapter of SessionSeatRepository on PostgreSQL, with visible SQL. */
@Repository
class JdbcSessionSeatRepository(
    private val jdbc: JdbcClient,
    private val publisher: ApplicationEventPublisher,
) : SessionSeatRepository {

    override fun isOnSale(session: SessionRef, now: Instant): Boolean? = jdbc.sql(
        """
        SELECT :now >= (e.sale_start AT TIME ZONE v.time_zone)
           AND :now <  (s.sale_end   AT TIME ZONE v.time_zone) AS on_sale
        FROM session s
        JOIN event e ON e.id = s.event_id
        JOIN venue v ON v.id = e.venue_id
        WHERE s.event_id = :eventId AND s.id = :sessionId
        """
    )
        .param("eventId", session.eventId)
        .param("sessionId", session.sessionId)
        .param("now", now.atOffset(ZoneOffset.UTC))
        .query { rs, _ -> rs.getBoolean("on_sale") }
        .optional().orElse(null)

    override fun find(id: SessionSeatId): SessionSeat? = jdbc.sql(
        """
        SELECT state, customer_id, hold_token, expires_at
        FROM session_seat
        WHERE $SEAT_KEY
        """
    )
        .seatParams(id)
        .query { rs, _ -> rs.toSessionSeat(id) }
        .optional().orElse(null)

    override fun countActiveHolds(customer: CustomerId, session: SessionRef, except: SeatRef, now: Instant): Int =
        jdbc.sql(
            """
            SELECT count(*)
            FROM session_seat
            WHERE event_id = :eventId AND session_id = :sessionId
              AND state = 'HELD' AND customer_id = :customer AND expires_at > :now
              AND NOT (sector_id = :sectorId AND row = :row AND number = :number)
            """
        )
            .seatParams(SessionSeatId(session, except))
            .param("customer", customer.value)
            .param("now", now.atOffset(ZoneOffset.UTC))
            .query(Long::class.java).single().toInt()

    override fun versionOf(session: SessionRef): Long? = jdbc.sql(
        """
        SELECT version
        FROM session
        WHERE event_id = :eventId AND id = :sessionId
        """
    )
        .param("eventId", session.eventId)
        .param("sessionId", session.sessionId)
        .query(Long::class.java)
        .optional().orElse(null)

    override fun seatsOf(session: SessionRef, now: Instant): List<SeatMapSeat> = jdbc.sql(
        """
        SELECT event_id, session_id, sector_id, row, number, price_tier_id, state, expires_at
        FROM session_seat
        WHERE event_id = :eventId AND session_id = :sessionId
        ORDER BY sector_id, row, number
        """
    )
        .param("eventId", session.eventId)
        .param("sessionId", session.sessionId)
        .param("now", now.atOffset(ZoneOffset.UTC))
        .query { rs, _ -> rs.toSeatMapSeat(now) }
        .list()

    /** Bumps the version of the seat map, in the same transaction as the change that made it stale. */
    private fun bumpVersion(session: SessionRef): Long {
        jdbc.sql(
            """
            UPDATE session SET version = version + 1
            WHERE event_id = :eventId AND id = :sessionId
            """
        )
            .param("eventId", session.eventId)
            .param("sessionId", session.sessionId)
            .update()
        return versionOf(session) ?: 0L
    }

    /** The UPDATE of ADR-0003; RETURNING gives back the new token and expiry in the same statement. */
    override fun tryHold(id: SessionSeatId, customer: CustomerId, ttl: Duration): SessionSeat.Held? {
        val held = jdbc.sql(
            """
            UPDATE session_seat
            SET state = 'HELD',
                customer_id = :customer,
                expires_at = now() + :ttlSeconds * interval '1 second',
                hold_token = gen_random_uuid()
            WHERE $SEAT_KEY
              AND (state = 'FREE' OR (state = 'HELD' AND expires_at < now()))
            RETURNING customer_id, hold_token, expires_at
            """
        )
            .seatParams(id)
            .param("customer", customer.value)
            .param("ttlSeconds", ttl.toSeconds())
            .query { rs, _ -> SessionSeat.Held(id, rs.toHold()) }
            .optional().orElse(null)
        if (held != null) {
            val newVersion = bumpVersion(id.session)
            publisher.publishEvent(SeatStateChangedEvent(id.session, id.seat, SeatStatus.HELD, newVersion))
        }
        return held
    }

    /** ADR-0006: frees the seat only if it is held by this customer with this token. */
    override fun release(id: SessionSeatId, customer: CustomerId, token: UUID): Boolean {
        val changed = jdbc.sql(
            """
            UPDATE session_seat
            SET state = 'FREE', customer_id = NULL, expires_at = NULL, hold_token = NULL
            WHERE $SEAT_KEY
              AND state = 'HELD' AND hold_token = :token AND customer_id = :customer
            """
        )
            .seatParams(id)
            .param("token", token)
            .param("customer", customer.value)
            .update() == 1
        if (changed) {
            val newVersion = bumpVersion(id.session)
            publisher.publishEvent(SeatStateChangedEvent(id.session, id.seat, SeatStatus.FREE, newVersion))
        }
        return changed
    }

    override fun activeHolds(customer: CustomerId, now: Instant): List<SessionSeat.Held> = jdbc.sql(
        """
        SELECT event_id, session_id, sector_id, row, number, customer_id, hold_token, expires_at
        FROM session_seat
        WHERE customer_id = :customer AND state = 'HELD' AND expires_at > :now
        ORDER BY expires_at
        """
    )
        .param("customer", customer.value)
        .param("now", now.atOffset(ZoneOffset.UTC))
        .query { rs, _ -> SessionSeat.Held(rs.toSessionSeatId(), rs.toHold()) }
        .list()
}

// ---- Helpers: parameters and rows ↔ domain types ----

/** Binds the parameters of SEAT_KEY. */
private fun JdbcClient.StatementSpec.seatParams(id: SessionSeatId): JdbcClient.StatementSpec =
    param("eventId", id.session.eventId)
        .param("sessionId", id.session.sessionId)
        .param("sectorId", id.seat.sectorId)
        .param("row", id.seat.row)
        .param("number", id.seat.number)

/** The CHECK of V1 allows only these three states, and guarantees the hold columns when HELD. */
private fun ResultSet.toSessionSeat(id: SessionSeatId): SessionSeat = when (val state = getString("state")) {
    "FREE" -> SessionSeat.Free(id)
    "SOLD" -> SessionSeat.Sold(id)
    "HELD" -> SessionSeat.Held(id, toHold())
    else -> error("unknown state of session_seat: $state")
}

/** One row of the seat map; an expired hold reads FREE (ADR-0003). */
private fun ResultSet.toSeatMapSeat(now: Instant): SeatMapSeat {
    val id = SessionSeatId(
        SessionRef(getLong("event_id"), getLong("session_id")),
        SeatRef(getLong("sector_id"), getString("row"), getInt("number")),
    )
    return SeatMapSeat(
        id = id,
        priceTierId = getLong("price_tier_id"),
        status = when (val state = getString("state")) {
            "FREE" -> SeatStatus.FREE
            "SOLD" -> SeatStatus.SOLD
            "HELD" ->
                if (getObject("expires_at", OffsetDateTime::class.java).toInstant().isAfter(now)) SeatStatus.HELD
                else SeatStatus.FREE
            else -> error("unknown state of session_seat: $state")
        },
    )
}

private fun ResultSet.toHold(): Hold = Hold(
    customer = CustomerId(getObject("customer_id", UUID::class.java)),
    token = getObject("hold_token", UUID::class.java),
    expiresAt = getObject("expires_at", OffsetDateTime::class.java).toInstant(),
)

private fun ResultSet.toSessionSeatId(): SessionSeatId = SessionSeatId(
    SessionRef(getLong("event_id"), getLong("session_id")),
    SeatRef(getLong("sector_id"), getString("row"), getInt("number")),
)