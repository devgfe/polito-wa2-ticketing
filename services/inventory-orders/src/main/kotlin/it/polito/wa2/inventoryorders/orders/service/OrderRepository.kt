package it.polito.wa2.inventoryorders.orders.service

import it.polito.wa2.inventoryorders.common.CustomerId
import it.polito.wa2.inventoryorders.orders.domain.Order
import it.polito.wa2.inventoryorders.orders.domain.OrderCursor
import it.polito.wa2.inventoryorders.orders.domain.OrderDraft
import java.time.Instant
import java.time.Duration
import java.util.UUID

/** An order found through its Idempotency-Key, with the hash of the request that created it (ADR-0005). */
data class KeyedOrder(val requestHash: String, val order: Order)

/** Port on the tables orders, order_item and idempotency_key. */
interface OrderRepository {

    /** The order created with this key by this customer; null if the key was never used (or expired and deleted). */
    fun findByIdempotencyKey(customer: CustomerId, key: UUID): KeyedOrder?

    /**
     * Saves a new order, its items and its Idempotency-Key, valid for `keyTtl` from the order (ADR-0005),
     * and returns the saved order, with its id and in state PENDING.
     * Throws DataIntegrityViolationException, inside this call, if the customer has already used the key.
     */
    fun create(draft: OrderDraft, key: UUID, requestHash: String, keyTtl: Duration): Order

    /** The orders of the customer, newest first, after the cursor (null: the first page), at most `size`. */
    fun page(customer: CustomerId, after: OrderCursor?, size: Int): List<Order>

    /** The order by its id, or null if it does not exist. */
    fun findById(orderId: Long): Order?

    /** Cancels a paid order: sets status to CANCELLED, frees its seats to FREE, bumps version. */
    fun cancel(customer: CustomerId, orderId: Long, reason: String, now: Instant): Order
}