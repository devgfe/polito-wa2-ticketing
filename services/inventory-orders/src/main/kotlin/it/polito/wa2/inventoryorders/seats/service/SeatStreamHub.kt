package it.polito.wa2.inventoryorders.seats.service

import it.polito.wa2.inventoryorders.common.SessionRef
import it.polito.wa2.inventoryorders.seats.domain.SeatChange
import it.polito.wa2.inventoryorders.seats.domain.SeatStateChangedEvent
import org.springframework.context.event.EventListener
import org.springframework.stereotype.Component
import org.springframework.web.servlet.mvc.method.annotation.SseEmitter
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.CopyOnWriteArrayList

@Component
class SeatStreamHub {
    private val emitters = ConcurrentHashMap<SessionRef, MutableList<SseEmitter>>()

    fun register(session: SessionRef, emitter: SseEmitter) {
        emitters.computeIfAbsent(session) { CopyOnWriteArrayList() }.add(emitter)
        emitter.onCompletion { remove(session, emitter) }
        emitter.onTimeout { remove(session, emitter) }
        emitter.onError { remove(session, emitter) }
    }

    private fun remove(session: SessionRef, emitter: SseEmitter) {
        emitters[session]?.remove(emitter)
    }

    @EventListener
    fun onSeatStateChanged(event: SeatStateChangedEvent) {
        val list = emitters[event.session] ?: return
        val change = SeatChange(event.seat, event.status)
        val dead = mutableListOf<SseEmitter>()
        for (emitter in list) {
            try {
                emitter.send(
                    SseEmitter.event()
                        .id(event.version.toString())
                        .name("seat")
                        .data(change)
                )
            } catch (ex: Exception) {
                dead.add(emitter)
            }
        }
        if (dead.isNotEmpty()) {
            list.removeAll(dead)
        }
    }
}
