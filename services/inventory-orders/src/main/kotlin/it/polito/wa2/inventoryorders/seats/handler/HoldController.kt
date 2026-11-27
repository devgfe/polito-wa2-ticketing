package it.polito.wa2.inventoryorders.seats.handler

import it.polito.wa2.inventoryorders.common.CUSTOMER_HEADER
import it.polito.wa2.inventoryorders.common.CustomerId
import it.polito.wa2.inventoryorders.common.DEFAULT_CUSTOMER_ID
import it.polito.wa2.inventoryorders.common.SeatRef
import it.polito.wa2.inventoryorders.common.SessionRef
import it.polito.wa2.inventoryorders.common.SessionSeatId
import it.polito.wa2.inventoryorders.errors.MalformedRequest
import it.polito.wa2.inventoryorders.seats.service.HoldService
import org.springframework.http.ResponseEntity
import org.springframework.web.bind.annotation.DeleteMapping
import org.springframework.web.bind.annotation.GetMapping
import org.springframework.web.bind.annotation.PathVariable
import org.springframework.web.bind.annotation.PostMapping
import org.springframework.web.bind.annotation.RequestHeader
import org.springframework.web.bind.annotation.RequestMapping
import org.springframework.web.bind.annotation.RestController
import org.springframework.web.util.UriComponentsBuilder
import java.net.URI
import java.util.UUID

/** The hold is a sub-resource of the seat of a session (ADR-0006). */
private const val HOLD_PATH = "/events/{eventId}/sessions/{sessionId}/seats/{sectorId}/{row}/{number}/hold"

@RestController
@RequestMapping("/api/v1")
class HoldController(private val holds: HoldService) {

    /** holdSeat: 201 if held now, 200 if the caller already held it (ADR-0006). */
    @PostMapping(HOLD_PATH)
    fun hold(
        @PathVariable eventId: Long,
        @PathVariable sessionId: Long,
        @PathVariable sectorId: Long,
        @PathVariable row: String,
        @PathVariable number: Int,
        @RequestHeader(name = CUSTOMER_HEADER, defaultValue = DEFAULT_CUSTOMER_ID) customer: UUID,
    ): ResponseEntity<HoldResponse> {
        val id = sessionSeatId(eventId, sessionId, sectorId, row, number)
        val result = holds.placeHold(id, CustomerId(customer))

        val response = if (result.created) ResponseEntity.created(holdUri(id)) else ResponseEntity.ok()
        return response
            .eTag(result.seat.hold.token.toString())
            .body(result.seat.toResponse())
    }

    /** getHold: the hold of the caller, with its token as ETag. */
    @GetMapping(HOLD_PATH)
    fun get(
        @PathVariable eventId: Long,
        @PathVariable sessionId: Long,
        @PathVariable sectorId: Long,
        @PathVariable row: String,
        @PathVariable number: Int,
        @RequestHeader(name = CUSTOMER_HEADER, defaultValue = DEFAULT_CUSTOMER_ID) customer: UUID,
    ): ResponseEntity<HoldResponse> {
        val held = holds.getHold(sessionSeatId(eventId, sessionId, sectorId, row, number), CustomerId(customer))
        return ResponseEntity.ok()
            .eTag(held.hold.token.toString())
            .body(held.toResponse())
    }

    /** releaseSeat: If-Match with the token is required (ADR-0006). */
    @DeleteMapping(HOLD_PATH)
    fun release(
        @PathVariable eventId: Long,
        @PathVariable sessionId: Long,
        @PathVariable sectorId: Long,
        @PathVariable row: String,
        @PathVariable number: Int,
        @RequestHeader(name = "If-Match") ifMatch: String,
        @RequestHeader(name = CUSTOMER_HEADER, defaultValue = DEFAULT_CUSTOMER_ID) customer: UUID,
    ): ResponseEntity<Void> {
        val id = sessionSeatId(eventId, sessionId, sectorId, row, number)
        holds.releaseHold(id, CustomerId(customer), tokenOf(ifMatch))
        return ResponseEntity.noContent().build()
    }

    /** listHolds: the active holds of the caller, in every session. */
    @GetMapping("/holds")
    fun list(
        @RequestHeader(name = CUSTOMER_HEADER, defaultValue = DEFAULT_CUSTOMER_ID) customer: UUID,
    ): HoldListResponse =
        HoldListResponse(holds.listHolds(CustomerId(customer)).map { it.toResponse() })

    /** Path values → domain types; a value that breaks their rules is a malformed request (400). */
    private fun sessionSeatId(eventId: Long, sessionId: Long, sectorId: Long, row: String, number: Int): SessionSeatId =
        try {
            SessionSeatId(SessionRef(eventId, sessionId), SeatRef(sectorId, row, number))
        } catch (ex: IllegalArgumentException) {
            throw MalformedRequest(ex.message ?: "invalid seat")
        }

    /** If-Match carries the token of the hold as an ETag, between double quotes: "7c9e6679-…". */
    private fun tokenOf(ifMatch: String): UUID {
        val token = ifMatch.removeSurrounding("\"")                         // "7c9e…" → 7c9e…
        return try {
            UUID.fromString(token)
        } catch (ex: IllegalArgumentException) {
            throw MalformedRequest("If-Match is not the ETag of a hold")    // 400
        }
    }

    /** Location of a new hold: the URI of the hold itself, relative to the gateway. */
    private fun holdUri(id: SessionSeatId): URI =
        UriComponentsBuilder.fromPath("/api/v1$HOLD_PATH")
            .encode()
            .buildAndExpand(id.session.eventId, id.session.sessionId, id.seat.sectorId, id.seat.row, id.seat.number)
            .toUri()
}