package it.polito.wa2.inventoryorders.orders.domain

import it.polito.wa2.inventoryorders.common.CustomerId
import it.polito.wa2.inventoryorders.common.SeatRef
import it.polito.wa2.inventoryorders.common.SessionRef
import it.polito.wa2.inventoryorders.common.MAX_SEATS_PER_CUSTOMER
import java.math.BigDecimal
import java.time.Instant

/** An amount of money: integer cents in memory and in the database, never floating point (ADR-0005). */
data class Money(val cents: Long, val currency: String) {
    init {
        require(cents >= 0) { "an amount cannot be negative" }
        require(currency.matches(Regex("[A-Z]{3}"))) { "currency must be an ISO 4217 code" }
    }

    /** The decimal string of the wire: 4500 → "45.00" (ADR-0005; two decimals, as EUR). */
    fun toDecimalString(): String = BigDecimal.valueOf(cents, 2).toPlainString()
}

/** Who will use the ticket of a seat (schemas FirstName, LastName). */
data class Holder(val firstName: String, val lastName: String) {
    init {
        require(firstName.length in 1..100) { "firstName must have 1 to 100 characters" }
        require(lastName.length in 1..100) { "lastName must have 1 to 100 characters" }
    }
}

/** One seat of an order, with its price in cents. */
data class OrderItem(val seat: SeatRef, val holder: Holder, val price: Long) {
    init {
        require(price >= 0) { "price cannot be negative" }
    }
}

/** Why an order was never paid (schema OrderFailureReason). */
enum class FailureReason { HOLD_EXPIRED, PAYMENT_DECLINED, PAYMENT_EXPIRED }

/** The state of an order, with the data that only that state has. */
sealed interface OrderState {
    /** Waiting for the payment; paymentUrl is null until createPayment answers (ADR-0004). */
    data class Pending(val paymentUrl: String?) : OrderState

    data class Paid(val paidAt: Instant) : OrderState

    data class Failed(val failedAt: Instant, val reason: FailureReason) : OrderState

    data class Cancelled(val paidAt: Instant, val cancelledAt: Instant, val reason: String) : OrderState
}

/** An order: the seats of one session bought by one customer (ADR-0005). */
data class Order(
    val id: Long,
    val session: SessionRef,
    val customer: CustomerId,
    val customerEmail: String,
    val currency: String,
    val orderedAt: Instant,
    val items: List<OrderItem>,
    val state: OrderState,
) {
    val total: Money get() = Money(items.sumOf { it.price }, currency)
}

/**
 * An order ready to be saved: everything but the id, which the database generates.
 * A new order is always PENDING, without a payment link yet.
 */
data class OrderDraft(
    val session: SessionRef,
    val customer: CustomerId,
    val customerEmail: String,
    val currency: String,
    val orderedAt: Instant,
    val items: List<OrderItem>,
)

/** The input of createOrder: 1 to MAX_SEATS_PER_CUSTOMER different seats of one session (ADR-0003). */
data class NewOrder(val session: SessionRef, val customerEmail: String, val items: List<NewOrderItem>) {
    init {
        require(items.size in 1..MAX_SEATS_PER_CUSTOMER) { "an order has 1 to $MAX_SEATS_PER_CUSTOMER seats" }
        require(items.map { it.seat }.distinct().size == items.size) { "a seat is listed more than once" }
    }
}

data class NewOrderItem(val seat: SeatRef, val holder: Holder)

/** Position in the list of orders of a customer, newest first: the last order of the previous page. */
data class OrderCursor(val orderedAt: Instant, val id: Long)