package it.polito.wa2.inventoryorders.seats.domain

import it.polito.wa2.inventoryorders.common.SeatRef
import it.polito.wa2.inventoryorders.common.SessionRef
import it.polito.wa2.inventoryorders.common.SessionSeatId

/** The three states of a seat on the wire (schema SeatStatus of openapi.yaml). */
enum class SeatStatus { FREE, HELD, SOLD }

/** One seat of the seat map: its price tier and its state at the moment the map was read. */
data class SeatMapSeat(
    val id: SessionSeatId,
    val priceTierId: Long,
    val status: SeatStatus,
)

/** The seat map of a session: its version, the ETag of the responses, and every seat. */
data class SeatMap(
    val version: Long,
    val seats: List<SeatMapSeat>,
)

/** Change of a seat state (schema SeatChange of openapi.yaml). */
data class SeatChange(
    val seat: SeatRef,
    val status: SeatStatus,
)

/** Event published when a seat state changes, for real-time SSE streaming. */
data class SeatStateChangedEvent(
    val session: SessionRef,
    val seat: SeatRef,
    val status: SeatStatus,
    val version: Long,
)
