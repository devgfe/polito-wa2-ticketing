package it.polito.wa2.inventoryorders.common

/** SessionRef on the wire: identifiers are strings in the contract. */
data class SessionRefDto(val eventId: String, val sessionId: String)

fun SessionRef.toDto() = SessionRefDto(eventId.toString(), sessionId.toString())

/** SeatRef on the wire, in the responses: identifiers are strings in the contract. */
data class SeatRefDto(val sectorId: String, val row: String, val number: Int)

fun SeatRef.toDto() = SeatRefDto(sectorId.toString(), row, number)