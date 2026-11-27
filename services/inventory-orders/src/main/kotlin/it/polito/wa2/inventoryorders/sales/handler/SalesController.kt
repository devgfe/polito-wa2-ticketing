package it.polito.wa2.inventoryorders.sales.handler

import it.polito.wa2.inventoryorders.common.DEFAULT_ORGANIZER_ID
import it.polito.wa2.inventoryorders.common.ORGANIZER_HEADER
import it.polito.wa2.inventoryorders.sales.service.SalesService
import org.springframework.http.ResponseEntity
import org.springframework.web.bind.annotation.GetMapping
import org.springframework.web.bind.annotation.PathVariable
import org.springframework.web.bind.annotation.RequestHeader
import org.springframework.web.bind.annotation.RequestMapping
import org.springframework.web.bind.annotation.RestController
import java.util.UUID

@RestController
@RequestMapping("/api/v1")
class SalesController(private val sales: SalesService) {

    @GetMapping("/events/{eventId}/sales")
    fun getSales(
        @PathVariable eventId: Long,
        @RequestHeader(name = ORGANIZER_HEADER, defaultValue = DEFAULT_ORGANIZER_ID) organizerId: UUID,
    ): ResponseEntity<EventSalesResponse> {
        val result = sales.getEventSales(eventId, organizerId)
        return ResponseEntity.ok(result.toResponse())
    }
}
