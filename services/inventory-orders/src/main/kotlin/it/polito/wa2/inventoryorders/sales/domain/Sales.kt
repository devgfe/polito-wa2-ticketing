package it.polito.wa2.inventoryorders.sales.domain

import it.polito.wa2.inventoryorders.orders.domain.Money
import java.util.UUID

data class PriceTierSales(
    val priceTierId: Long,
    val price: Money,
    val onSale: Int,
    val sold: Int,
    val total: Money,
)

data class SessionSales(
    val sessionId: Long,
    val sold: Int,
    val total: Money,
    val priceTiers: List<PriceTierSales>,
)

data class EventSales(
    val sold: Int,
    val total: Money,
    val sessions: List<SessionSales>,
)

data class EventInfo(
    val organizerId: UUID,
    val currency: String,
)
