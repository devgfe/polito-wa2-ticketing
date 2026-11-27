package it.polito.wa2.inventoryorders.seats.handler

import it.polito.wa2.inventoryorders.common.SessionRef
import it.polito.wa2.inventoryorders.errors.MalformedRequest
import it.polito.wa2.inventoryorders.seats.service.SeatMapService
import org.springframework.http.HttpStatus
import org.springframework.http.ResponseEntity
import org.springframework.web.bind.annotation.GetMapping
import org.springframework.web.bind.annotation.PathVariable
import org.springframework.web.bind.annotation.RequestHeader
import org.springframework.web.bind.annotation.RequestMapping
import org.springframework.web.bind.annotation.RestController

/**
 * The seat map of a session, public: no token, no customer (security: [] of openapi.yaml).
 */
@RestController
@RequestMapping("/api/v1")
class SeatMapController(private val maps: SeatMapService) {

    /** getSeatMap: the map with its ETag (the version), or 304 when If-None-Match matches it. */
    @GetMapping("/events/{eventId}/sessions/{sessionId}/seats")
    fun map(
        @PathVariable eventId: Long,
        @PathVariable sessionId: Long,
        @RequestHeader(value = "If-None-Match", required = false) ifNoneMatch: String?,
    ): ResponseEntity<SeatMapResponse> {
        val map = maps.seatMap(sessionOf(eventId, sessionId))
        val etag = map.version.toString()

        if (ifNoneMatch != null && ifNoneMatch.removeSurrounding("\"") == etag) {
            return ResponseEntity.status(HttpStatus.NOT_MODIFIED).eTag(etag).build()   // no body, ETag kept
        }
        return ResponseEntity.ok()
            .eTag(etag)
            .body(map.toResponse())
    }

    /** Path values → domain type; a value that breaks its rules is a malformed request (400). */
    private fun sessionOf(eventId: Long, sessionId: Long): SessionRef =
        try {
            SessionRef(eventId, sessionId)
        } catch (ex: IllegalArgumentException) {
            throw MalformedRequest(ex.message ?: "invalid session")
        }
}
