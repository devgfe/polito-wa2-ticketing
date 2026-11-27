package it.polito.wa2.inventoryorders.sales.store

import it.polito.wa2.inventoryorders.sales.domain.EventInfo
import it.polito.wa2.inventoryorders.sales.domain.EventSales
import it.polito.wa2.inventoryorders.sales.domain.PriceTierSales
import it.polito.wa2.inventoryorders.sales.domain.SessionSales
import it.polito.wa2.inventoryorders.orders.domain.Money
import org.springframework.jdbc.core.simple.JdbcClient
import org.springframework.stereotype.Repository
import java.util.UUID

interface SalesRepository {
    fun findEventInfo(eventId: Long): EventInfo?
    fun findEventSales(eventId: Long, currency: String): EventSales?
}

@Repository
class JdbcSalesRepository(private val jdbc: JdbcClient) : SalesRepository {

    override fun findEventInfo(eventId: Long): EventInfo? = jdbc.sql(
        """
        SELECT organizer_id, currency
        FROM event
        WHERE id = :eventId
        """
    )
        .param("eventId", eventId)
        .query { rs, _ ->
            EventInfo(
                organizerId = rs.getObject("organizer_id", UUID::class.java),
                currency = rs.getString("currency"),
            )
        }
        .optional().orElse(null)

    override fun findEventSales(eventId: Long, currency: String): EventSales? {
        // 1. Get all sessions for the event
        val sessionIds = jdbc.sql(
            """
            SELECT id FROM session WHERE event_id = :eventId ORDER BY id
            """
        )
            .param("eventId", eventId)
            .query(Long::class.java)
            .list()

        if (sessionIds.isEmpty()) {
            // Check if event exists at all
            val eventExists = jdbc.sql("SELECT 1 FROM event WHERE event_id = :eventId".replace("event_id", "id"))
                .param("eventId", eventId)
                .query(Int::class.java)
                .optional().isPresent
            if (!eventExists) return null
            return EventSales(0, Money(0, currency), emptyList())
        }

        val sessionSalesList = mutableListOf<SessionSales>()
        var totalEventSold = 0
        var totalEventCents = 0L

        for (sessionId in sessionIds) {
            // Get price tiers for this session
            val priceTiers = jdbc.sql(
                """
                SELECT p.id as price_tier_id, p.price,
                       (SELECT count(*) FROM session_seat ss WHERE ss.event_id = :eventId AND ss.session_id = :sessionId AND ss.price_tier_id = p.id) as on_sale,
                       (SELECT count(*) FROM session_seat ss WHERE ss.event_id = :eventId AND ss.session_id = :sessionId AND ss.price_tier_id = p.id AND ss.state = 'SOLD') as sold
                FROM price_tier p
                WHERE p.event_id = :eventId AND p.session_id = :sessionId
                ORDER BY p.id
                """
            )
                .param("eventId", eventId)
                .param("sessionId", sessionId)
                .query { rs, _ ->
                    val price = rs.getLong("price")
                    val onSale = rs.getInt("on_sale")
                    val sold = rs.getInt("sold")
                    val tierTotalCents = sold * price
                    PriceTierSales(
                        priceTierId = rs.getLong("price_tier_id"),
                        price = Money(price, currency),
                        onSale = onSale,
                        sold = sold,
                        total = Money(tierTotalCents, currency),
                    )
                }
                .list()

            val sessionSold = priceTiers.sumOf { it.sold }
            val sessionCents = priceTiers.sumOf { it.total.cents }

            totalEventSold += sessionSold
            totalEventCents += sessionCents

            sessionSalesList.add(
                SessionSales(
                    sessionId = sessionId!!,
                    sold = sessionSold,
                    total = Money(sessionCents, currency),
                    priceTiers = priceTiers,
                )
            )
        }

        return EventSales(
            sold = totalEventSold,
            total = Money(totalEventCents, currency),
            sessions = sessionSalesList,
        )
    }
}
