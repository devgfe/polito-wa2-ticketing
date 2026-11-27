package it.polito.wa2.inventoryorders.sales.service

import it.polito.wa2.inventoryorders.errors.Forbidden
import it.polito.wa2.inventoryorders.errors.NotFound
import it.polito.wa2.inventoryorders.sales.domain.EventSales
import it.polito.wa2.inventoryorders.sales.store.SalesRepository
import it.polito.wa2.inventoryorders.orders.domain.Money
import org.springframework.stereotype.Service
import org.springframework.transaction.annotation.Transactional
import java.util.UUID

@Service
class SalesService(private val sales: SalesRepository) {

    @Transactional(readOnly = true)
    fun getEventSales(eventId: Long, organizerId: UUID): EventSales {
        val eventInfo = sales.findEventInfo(eventId) ?: throw NotFound("event $eventId")
        if (eventInfo.organizerId != organizerId) {
            throw Forbidden("event $eventId does not belong to organizer")
        }
        return sales.findEventSales(eventId, eventInfo.currency)
            ?: EventSales(0, Money(0, eventInfo.currency), emptyList())
    }
}
