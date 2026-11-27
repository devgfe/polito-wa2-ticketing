package it.polito.wa2.inventoryorders.sales.handler

import it.polito.wa2.inventoryorders.sales.domain.EventSales
import it.polito.wa2.inventoryorders.sales.domain.PriceTierSales
import it.polito.wa2.inventoryorders.sales.domain.SessionSales
import it.polito.wa2.inventoryorders.orders.domain.Money

data class MoneyDto(val amount: String, val currency: String)

data class PriceTierSalesResponse(
    val priceTierId: String,
    val price: MoneyDto,
    val onSale: Int,
    val sold: Int,
    val total: MoneyDto,
)

data class SessionSalesResponse(
    val sessionId: String,
    val sold: Int,
    val total: MoneyDto,
    val priceTiers: List<PriceTierSalesResponse>,
)

data class EventSalesResponse(
    val sold: Int,
    val total: MoneyDto,
    val sessions: List<SessionSalesResponse>,
)

fun Money.toDto() = MoneyDto(toDecimalString(), currency)

fun PriceTierSales.toResponse() = PriceTierSalesResponse(
    priceTierId = priceTierId.toString(),
    price = price.toDto(),
    onSale = onSale,
    sold = sold,
    total = total.toDto(),
)

fun SessionSales.toResponse() = SessionSalesResponse(
    sessionId = sessionId.toString(),
    sold = sold,
    total = total.toDto(),
    priceTiers = priceTiers.map { it.toResponse() },
)

fun EventSales.toResponse() = EventSalesResponse(
    sold = sold,
    total = total.toDto(),
    sessions = sessions.map { it.toResponse() },
)
