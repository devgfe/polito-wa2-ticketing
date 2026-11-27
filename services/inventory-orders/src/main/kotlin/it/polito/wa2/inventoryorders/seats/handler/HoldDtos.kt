package it.polito.wa2.inventoryorders.seats.handler

import it.polito.wa2.inventoryorders.common.SeatRefDto
import it.polito.wa2.inventoryorders.common.SessionRefDto
import it.polito.wa2.inventoryorders.common.toDto
import it.polito.wa2.inventoryorders.seats.domain.SessionSeat
import java.time.Instant
import java.util.UUID

/** A hold in the responses: schema Hold of openapi.yaml. */
data class HoldResponse(
    val session: SessionRefDto,
    val seat: SeatRefDto,
    val token: UUID,
    val expiresAt: Instant,
)

/** Response of GET /holds: schema HoldList of openapi.yaml. */
data class HoldListResponse(val items: List<HoldResponse>)

fun SessionSeat.Held.toResponse() = HoldResponse(
    session = id.session.toDto(),
    seat = id.seat.toDto(),
    token = hold.token,
    expiresAt = hold.expiresAt,
)