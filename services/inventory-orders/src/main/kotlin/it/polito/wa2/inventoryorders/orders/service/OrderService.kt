package it.polito.wa2.inventoryorders.orders.service

import it.polito.wa2.inventoryorders.common.CustomerId
import it.polito.wa2.inventoryorders.config.OrdersProperties
import it.polito.wa2.inventoryorders.errors.IdempotencyKeyInUse
import it.polito.wa2.inventoryorders.errors.IdempotencyKeyReused
import it.polito.wa2.inventoryorders.errors.NotFound
import it.polito.wa2.inventoryorders.errors.SeatNotHeld
import it.polito.wa2.inventoryorders.orders.domain.NewOrder
import it.polito.wa2.inventoryorders.orders.domain.Order
import it.polito.wa2.inventoryorders.orders.domain.OrderCursor
import it.polito.wa2.inventoryorders.orders.domain.OrderDraft
import it.polito.wa2.inventoryorders.orders.domain.OrderItem
import org.springframework.dao.DataIntegrityViolationException
import org.springframework.stereotype.Service
import org.springframework.transaction.annotation.Transactional
import java.security.MessageDigest
import java.time.Clock
import java.time.temporal.ChronoUnit
import java.util.HexFormat
import java.util.UUID

/** A page of orders, and the cursor of the next page (null on the last one). */
data class OrderPage(val orders: List<Order>, val next: OrderCursor?)

/** The use cases of the orders: create, list (ADR-0005). */
@Service
class OrderService(
    private val orders: OrderRepository,
    private val heldSeats: HeldSeatRepository,
    private val clock: Clock,
    private val props: OrdersProperties,
) {

    /** A new PENDING order with seats held by the customer, or the order already created with this key. */
    @Transactional
    fun createOrder(customer: CustomerId, key: UUID, input: NewOrder): Order {
        val requestHash = requestHash(input)

        // 1. Key already used: same request → the saved order; different request → 422 (ADR-0005)
        val saved = orders.findByIdempotencyKey(customer, key)
        if (saved != null) {
            if (saved.requestHash != requestHash) throw IdempotencyKeyReused()
            // TODO(saga): if the order is still PENDING without a payment, call createPayment again (ADR-0004)
            return saved.order
        }

        // 2. Every seat must be held by the customer; seats are locked in one fixed order
        val now = clock.instant().truncatedTo(ChronoUnit.MICROS)     // PostgreSQL keeps microseconds
        val seats = input.items
            .sortedWith(compareBy({ it.seat.sectorId }, { it.seat.row }, { it.seat.number }))
            .map { item ->
                val held = heldSeats.lockHeldSeat(input.session, item.seat, customer, now)
                    ?: throw SeatNotHeld(item.seat)
                item to held
            }
        // TODO(saga): a seat may already be in another PENDING order of the same customer (open point)

        val draft = OrderDraft(
            session = input.session,
            customer = customer,
            customerEmail = input.customerEmail,
            currency = seats.first().second.price.currency,     // the same for every seat: one event, one currency
            orderedAt = now,
            items = seats.map { (item, held) -> OrderItem(item.seat, item.holder, held.price.cents) },
        )

        // 3. Save order and key; a parallel request with the same key loses here (Lab03, task 4)
        val order = try {
            orders.create(draft, key, requestHash, props.idempotencyKeyTtl)
        } catch (ex: DataIntegrityViolationException) {
            throw IdempotencyKeyInUse()
        }

        // TODO(saga): call createPayment outside this transaction and store the PayPal link (ADR-0004, ADR-0005)
        return order
    }

    /** The orders of the customer, newest first, one page at a time (L18, keyset). */
    @Transactional(readOnly = true)
    fun listOrders(customer: CustomerId, after: OrderCursor?, limit: Int): OrderPage {
        val rows = orders.page(customer, after, limit + 1)           // one more: is there a next page?
        val page = rows.take(limit)
        val next = if (rows.size > limit) OrderCursor(page.last().orderedAt, page.last().id) else null
        return OrderPage(page, next)
    }

    /** An order of the customer by its id; 404 if it does not exist or belongs to another customer. */
    @Transactional(readOnly = true)
    fun getOrder(customer: CustomerId, orderId: Long): Order {
        val order = orders.findById(orderId) ?: throw NotFound("order $orderId")
        if (order.customer != customer) throw NotFound("order $orderId")
        return order
    }

    /** Cancels a paid order of the customer. */
    @Transactional
    fun cancelOrder(customer: CustomerId, orderId: Long, reason: String): Order {
        val now = clock.instant().truncatedTo(ChronoUnit.MICROS)
        return orders.cancel(customer, orderId, reason, now)
    }

    /** Fingerprint of the body, to recognise a key reused for a different purchase (ADR-0005). */
    private fun requestHash(input: NewOrder): String {
        val text = buildString {
            append(input.session.eventId).append('|').append(input.session.sessionId)
            append('|').append(input.customerEmail)
            input.items.forEach {
                append('|').append(it.seat).append('|').append(it.holder.firstName).append('|').append(it.holder.lastName)
            }
        }
        return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(text.toByteArray()))
    }
}