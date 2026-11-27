package it.polito.wa2.inventoryorders.orders.handler

import it.polito.wa2.inventoryorders.common.SeatRefDto
import it.polito.wa2.inventoryorders.common.SessionRefDto
import it.polito.wa2.inventoryorders.common.SeatRef
import it.polito.wa2.inventoryorders.common.SessionRef
import it.polito.wa2.inventoryorders.orders.domain.Holder
import it.polito.wa2.inventoryorders.orders.domain.Money
import it.polito.wa2.inventoryorders.orders.domain.NewOrder
import it.polito.wa2.inventoryorders.orders.domain.NewOrderItem
import it.polito.wa2.inventoryorders.orders.domain.Order
import it.polito.wa2.inventoryorders.orders.domain.OrderItem
import it.polito.wa2.inventoryorders.orders.domain.OrderState
import jakarta.validation.Valid
import jakarta.validation.constraints.Email
import jakarta.validation.constraints.Min
import jakarta.validation.constraints.NotBlank
import jakarta.validation.constraints.NotNull
import jakarta.validation.constraints.Size
import java.time.Instant

data class MoneyResponse(val amount: String, val currency: String)

data class HolderDto(
    @field:NotBlank @field:Size(min = 1, max = 100) val firstName: String,
    @field:NotBlank @field:Size(min = 1, max = 100) val lastName: String,
)

data class SeatRefInput(
    @field:NotNull val sectorId: Long,
    @field:NotBlank val row: String,
    @field:NotNull @field:Min(1) val number: Int,
)

data class SessionRefInput(
    @field:NotNull val eventId: Long,
    @field:NotNull val sessionId: Long,
)

data class OrderItemInputDto(
    @field:NotNull @field:Valid val seat: SeatRefInput,
    @field:NotNull @field:Valid val holder: HolderDto,
)

data class OrderInputDto(
    @field:NotNull @field:Valid val session: SessionRefInput,
    @field:NotBlank @field:Email val customerEmail: String,
    @field:NotNull @field:Size(min = 1, max = 4) val items: List<OrderItemInputDto>,
)

data class OrderItemResponse(
    val seat: SeatRefDto,
    val holder: HolderDto,
    val price: MoneyResponse,
)

data class OrderResponse(
    val id: String,
    val session: SessionRefDto,
    val status: String,
    val orderedAt: Instant,
    val customerEmail: String,
    val total: MoneyResponse,
    val items: List<OrderItemResponse>,
    val paymentUrl: String? = null,
    val paidAt: Instant? = null,
    val failedAt: Instant? = null,
    val failureReason: String? = null,
    val cancelledAt: Instant? = null,
    val reason: String? = null,
)

data class OrderPageResponse(
    val items: List<OrderResponse>,
    val nextCursor: String?,
    val limit: Int,
)

data class CancellationInputDto(
    @field:NotBlank @field:Size(min = 1, max = 500) val reason: String,
)

fun OrderInputDto.toDomain() = NewOrder(
    session = SessionRef(session.eventId, session.sessionId),
    customerEmail = customerEmail,
    items = items.map {
        NewOrderItem(
            seat = SeatRef(it.seat.sectorId, it.seat.row, it.seat.number),
            holder = Holder(it.holder.firstName, it.holder.lastName),
        )
    }
)

fun moneyToDto(money: Money) = MoneyResponse(money.toDecimalString(), money.currency)

fun holderToDto(holder: Holder) = HolderDto(holder.firstName, holder.lastName)

fun OrderItem.toResponse(currency: String) = OrderItemResponse(
    seat = SeatRefDto(seat.sectorId.toString(), seat.row, seat.number),
    holder = holderToDto(holder),
    price = moneyToDto(Money(price, currency)),
)

fun Order.toResponse(): OrderResponse {
    var paymentUrl: String? = null
    var paidAt: Instant? = null
    var failedAt: Instant? = null
    var failureReason: String? = null
    var cancelledAt: Instant? = null
    var reason: String? = null

    when (val s = state) {
        is OrderState.Pending -> paymentUrl = s.paymentUrl
        is OrderState.Paid -> paidAt = s.paidAt
        is OrderState.Failed -> {
            failedAt = s.failedAt
            failureReason = s.reason.name
        }
        is OrderState.Cancelled -> {
            paidAt = s.paidAt
            cancelledAt = s.cancelledAt
            reason = s.reason
        }
    }

    return OrderResponse(
        id = id.toString(),
        session = SessionRefDto(session.eventId.toString(), session.sessionId.toString()),
        status = when (state) {
            is OrderState.Pending -> "PENDING"
            is OrderState.Paid -> "PAID"
            is OrderState.Failed -> "FAILED"
            is OrderState.Cancelled -> "CANCELLED"
        },
        orderedAt = orderedAt,
        customerEmail = customerEmail,
        total = moneyToDto(total),
        items = items.map { it.toResponse(currency) },
        paymentUrl = paymentUrl,
        paidAt = paidAt,
        failedAt = failedAt,
        failureReason = failureReason,
        cancelledAt = cancelledAt,
        reason = reason,
    )
}
