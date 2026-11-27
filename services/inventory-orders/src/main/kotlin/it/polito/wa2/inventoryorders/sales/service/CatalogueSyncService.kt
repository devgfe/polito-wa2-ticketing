package it.polito.wa2.inventoryorders.sales.service

import org.springframework.jdbc.core.simple.JdbcClient
import org.springframework.stereotype.Service
import org.springframework.transaction.annotation.Transactional
import org.springframework.web.client.RestClient
import org.springframework.web.client.body
import java.math.BigDecimal
import java.time.LocalDateTime
import java.util.UUID

@Service
class CatalogueSyncService(
    private val catalogueClient: RestClient,
    private val jdbc: JdbcClient,
) {

    @Transactional
    fun syncEvent(eventId: Long) {
        // 1. Fetch Event
        val event = catalogueClient.get()
            .uri("/events/{eventId}", eventId)
            .retrieve()
            .body<CatalogueEventDto>() ?: throw IllegalStateException("Event $eventId not found in Catalogue")

        val venueId = event.venue.id

        // 2. Fetch Venue
        val venue = catalogueClient.get()
            .uri("/venues/{venueId}", venueId)
            .retrieve()
            .body<CatalogueVenueDto>() ?: throw IllegalStateException("Venue $venueId not found in Catalogue")

        // Upsert Venue
        jdbc.sql("INSERT INTO venue (id, time_zone) VALUES (:id, :tz) ON CONFLICT (id) DO UPDATE SET time_zone = EXCLUDED.time_zone")
            .param("id", venue.id)
            .param("tz", venue.timeZone)
            .update()

        // Upsert Sectors & Seats
        for (sector in venue.sectors) {
            jdbc.sql("INSERT INTO sector (venue_id, id) VALUES (:venueId, :sectorId) ON CONFLICT (venue_id, id) DO NOTHING")
                .param("venueId", venue.id)
                .param("sectorId", sector.id)
                .update()

            for (seat in sector.seats) {
                jdbc.sql("INSERT INTO seat (venue_id, sector_id, row, number) VALUES (:venueId, :sectorId, :row, :number) ON CONFLICT (venue_id, sector_id, row, number) DO NOTHING")
                    .param("venueId", venue.id)
                    .param("sectorId", sector.id)
                    .param("row", seat.row)
                    .param("number", seat.number)
                    .update()
            }
        }

        // Upsert Event
        // Note: Catalogue event has organizerId or we default it if not present
        val organizerId = event.organizerId ?: UUID.fromString("22222222-2222-2222-2222-222222222222")
        jdbc.sql(
            """
            INSERT INTO event (id, venue_id, organizer_id, sale_start, currency)
            VALUES (:id, :venueId, :organizerId, :saleStart, :currency)
            ON CONFLICT (id) DO UPDATE SET venue_id = EXCLUDED.venue_id, sale_start = EXCLUDED.sale_start, currency = EXCLUDED.currency
            """
        )
            .param("id", event.id)
            .param("venueId", venue.id)
            .param("organizerId", organizerId)
            .param("saleStart", LocalDateTime.parse(event.saleStart))
            .param("currency", event.currency)
            .update()

        // 3. Fetch Sessions and Price Tiers
        for (sessionSummary in event.sessions) {
            val sessionId = sessionSummary.id
            val session = catalogueClient.get()
                .uri("/events/{eventId}/sessions/{sessionId}", eventId, sessionId)
                .retrieve()
                .body<CatalogueSessionDto>() ?: continue

            jdbc.sql(
                """
                INSERT INTO session (event_id, id, starts_at, sale_end)
                VALUES (:eventId, :sessionId, :startsAt, :saleEnd)
                ON CONFLICT (event_id, id) DO UPDATE SET starts_at = EXCLUDED.starts_at, sale_end = EXCLUDED.sale_end
                """
            )
                .param("eventId", eventId)
                .param("sessionId", sessionId)
                .param("startsAt", LocalDateTime.parse(session.startsAt))
                .param("saleEnd", LocalDateTime.parse(session.saleEnd))
                .update()

            for (pt in session.priceTiers) {
                val priceCents = BigDecimal(pt.price.amount).multiply(BigDecimal(100)).longValueExact()
                jdbc.sql(
                    """
                    INSERT INTO price_tier (event_id, session_id, id, price)
                    VALUES (:eventId, :sessionId, :ptId, :price)
                    ON CONFLICT (event_id, session_id, id) DO UPDATE SET price = EXCLUDED.price
                    """
                )
                    .param("eventId", eventId)
                    .param("sessionId", sessionId)
                    .param("ptId", pt.id)
                    .param("price", priceCents)
                    .update()

                for (sRef in pt.seats) {
                    jdbc.sql(
                        """
                        INSERT INTO session_seat (event_id, session_id, venue_id, sector_id, row, number, price_tier_id, state)
                        VALUES (:eventId, :sessionId, :venueId, :sectorId, :row, :number, :ptId, 'FREE')
                        ON CONFLICT (event_id, session_id, venue_id, sector_id, row, number)
                        DO UPDATE SET price_tier_id = EXCLUDED.price_tier_id
                        """
                    )
                        .param("eventId", eventId)
                        .param("sessionId", sessionId)
                        .param("venueId", venue.id)
                        .param("sectorId", sRef.sectorId)
                        .param("row", sRef.row)
                        .param("number", sRef.number)
                        .param("ptId", pt.id)
                        .update()
                }
            }
        }
    }
}

// DTOs for parsing Catalogue API responses
data class CatalogueEventDto(
    val id: Long,
    val name: String,
    val venue: CatalogueVenueSummaryDto,
    val currency: String,
    val saleStart: String,
    val organizerId: UUID?,
    val sessions: List<CatalogueSessionSummaryDto>,
)

data class CatalogueVenueSummaryDto(val id: Long, val name: String, val city: String)

data class CatalogueSessionSummaryDto(val id: Long, val startsAt: String, val endsAt: String, val saleEnd: String)

data class CatalogueVenueDto(
    val id: Long,
    val name: String,
    val city: String,
    val timeZone: String,
    val sectors: List<CatalogueSectorDto>,
)

data class CatalogueSectorDto(
    val id: Long,
    val name: String,
    val seats: List<CatalogueSeatDto>,
)

data class CatalogueSeatDto(
    val row: String,
    val number: Int,
)

data class CatalogueSessionDto(
    val id: Long,
    val timeZone: String,
    val startsAt: String,
    val endsAt: String,
    val saleEnd: String,
    val priceTiers: List<CataloguePriceTierDto>,
)

data class CataloguePriceTierDto(
    val id: Long,
    val name: String,
    val price: CatalogueMoneyDto,
    val seats: List<CatalogueSeatRefDto>,
)

data class CatalogueMoneyDto(val amount: String, val currency: String)

data class CatalogueSeatRefDto(val sectorId: Long, val row: String, val number: Int)
