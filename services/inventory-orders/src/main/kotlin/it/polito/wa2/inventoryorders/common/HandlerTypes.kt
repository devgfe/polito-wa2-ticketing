package it.polito.wa2.inventoryorders.common

/** SeatRef on the wire, in the responses: identifiers are strings in the contract. */
data class SeatRefDto(val sectorId: String, val row: String, val number: Int)

fun SeatRef.toDto() = SeatRefDto(sectorId.toString(), row, number)