package it.polito.wa2.inventoryorders.seats.handler

import it.polito.wa2.inventoryorders.common.SeatRefDto
import it.polito.wa2.inventoryorders.common.toDto
import it.polito.wa2.inventoryorders.seats.domain.SeatMap
import it.polito.wa2.inventoryorders.seats.domain.SeatStatus

/** One seat of the seat map on the wire: schema SeatState of openapi.yaml. */
data class SeatStateResponse(
    val seat: SeatRefDto,
    val priceTierId: String,
    val status: SeatStatus,
)

/** Response of GET /events/{eventId}/sessions/{sessionId}/seats: schema SeatMap of openapi.yaml. */
data class SeatMapResponse(val seats: List<SeatStateResponse>)

fun SeatMap.toResponse() = SeatMapResponse(
    seats = seats.map {
        SeatStateResponse(
            seat = it.id.seat.toDto(),
            priceTierId = it.priceTierId.toString(),
            status = it.status,
        )
    }
)
