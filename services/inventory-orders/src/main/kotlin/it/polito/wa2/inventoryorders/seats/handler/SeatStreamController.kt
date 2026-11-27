package it.polito.wa2.inventoryorders.seats.handler

import it.polito.wa2.inventoryorders.common.SessionRef
import it.polito.wa2.inventoryorders.errors.MalformedRequest
import it.polito.wa2.inventoryorders.seats.service.SeatMapService
import it.polito.wa2.inventoryorders.seats.service.SeatStreamHub
import org.springframework.http.MediaType
import org.springframework.web.bind.annotation.GetMapping
import org.springframework.web.bind.annotation.PathVariable
import org.springframework.web.bind.annotation.RequestMapping
import org.springframework.web.bind.annotation.RestController
import org.springframework.web.servlet.mvc.method.annotation.SseEmitter
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit

@RestController
@RequestMapping("/api/v1")
class SeatStreamController(
    private val maps: SeatMapService,
    private val hub: SeatStreamHub,
) {
    private val scheduler = Executors.newSingleThreadScheduledExecutor()

    @GetMapping("/events/{eventId}/sessions/{sessionId}/seat-stream", produces = [MediaType.TEXT_EVENT_STREAM_VALUE])
    fun stream(
        @PathVariable eventId: Long,
        @PathVariable sessionId: Long,
    ): SseEmitter {
        val session = sessionOf(eventId, sessionId)
        // 1. Get initial snapshot (validates session existence, throws 404 if not found)
        val map = maps.seatMap(session)

        val emitter = SseEmitter(30 * 60 * 1000L) // 30 minutes timeout

        // Send initial snapshot as event named "snapshot" with id = version
        try {
            emitter.send(
                SseEmitter.event()
                    .id(map.version.toString())
                    .name("snapshot")
                    .data(map.toResponse())
            )
        } catch (ex: Exception) {
            emitter.completeWithError(ex)
            return emitter
        }

        hub.register(session, emitter)

        // Heartbeat every 15s
        val heartbeat = scheduler.scheduleAtFixedRate({
            try {
                emitter.send(SseEmitter.event().comment("heartbeat"))
            } catch (ex: Exception) {
                emitter.complete()
            }
        }, 15, 15, TimeUnit.SECONDS)

        emitter.onCompletion { heartbeat.cancel(true) }
        emitter.onTimeout { heartbeat.cancel(true) }
        emitter.onError { heartbeat.cancel(true) }

        return emitter
    }

    private fun sessionOf(eventId: Long, sessionId: Long): SessionRef =
        try {
            SessionRef(eventId, sessionId)
        } catch (ex: IllegalArgumentException) {
            throw MalformedRequest(ex.message ?: "invalid session")
        }
}
