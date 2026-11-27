package it.polito.wa2.inventoryorders.seats.service

import it.polito.wa2.inventoryorders.common.SessionRef
import it.polito.wa2.inventoryorders.errors.NotFound
import it.polito.wa2.inventoryorders.seats.domain.SeatMap
import org.springframework.stereotype.Service
import org.springframework.transaction.annotation.Transactional
import java.time.Clock

/** The use case of the seat map: read it with its version, the ETag of the responses. */
@Service
class SeatMapService(
    private val seats: SessionSeatRepository,
    private val clock: Clock,
) {

    /** The seat map of the session; 404 if the session is not in the copies yet. */
    @Transactional(readOnly = true)
    fun seatMap(session: SessionRef): SeatMap {
        val version = seats.versionOf(session) ?: throw NotFound("seat map of $session")
        return SeatMap(version, seats.seatsOf(session, clock.instant()))
    }
}
