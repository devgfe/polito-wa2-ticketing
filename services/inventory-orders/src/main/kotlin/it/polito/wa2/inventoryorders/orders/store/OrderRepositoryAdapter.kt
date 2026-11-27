package it.polito.wa2.inventoryorders.orders.store

import it.polito.wa2.inventoryorders.common.CustomerId
import it.polito.wa2.inventoryorders.common.SeatRef
import it.polito.wa2.inventoryorders.common.SessionRef
import it.polito.wa2.inventoryorders.orders.domain.FailureReason
import it.polito.wa2.inventoryorders.orders.domain.Holder
import it.polito.wa2.inventoryorders.orders.domain.Order
import it.polito.wa2.inventoryorders.orders.domain.OrderCursor
import it.polito.wa2.inventoryorders.orders.domain.OrderDraft
import it.polito.wa2.inventoryorders.orders.domain.OrderItem
import it.polito.wa2.inventoryorders.orders.domain.OrderState
import it.polito.wa2.inventoryorders.orders.service.KeyedOrder
import it.polito.wa2.inventoryorders.orders.service.OrderRepository
import jakarta.persistence.EntityManager
import org.springframework.data.domain.Limit
import org.springframework.data.repository.findByIdOrNull
import org.springframework.stereotype.Repository
import it.polito.wa2.inventoryorders.errors.NotFound
import it.polito.wa2.inventoryorders.errors.OrderNotCancellable
import it.polito.wa2.inventoryorders.seats.domain.SeatStateChangedEvent
import it.polito.wa2.inventoryorders.seats.domain.SeatStatus
import org.springframework.context.ApplicationEventPublisher
import java.time.Duration
import java.time.Instant
import java.util.UUID

/** Adapter of OrderRepository on PostgreSQL with JPA (ADR-0002). */
@Repository
class OrderRepositoryAdapter(
    private val orders: OrderJpaRepository,
    private val items: OrderItemJpaRepository,
    private val keys: IdempotencyKeyJpaRepository,
    private val em: EntityManager,
    private val publisher: ApplicationEventPublisher,
) : OrderRepository {

    override fun findByIdempotencyKey(customer: CustomerId, key: UUID): KeyedOrder? {
        val saved = keys.findByIdOrNull(IdempotencyKeyId(customer.value, key)) ?: return null
        val order = orders.findByIdOrNull(saved.orderId)
            ?: error("order ${saved.orderId} of an Idempotency-Key not found")   // the foreign key forbids it
        return KeyedOrder(saved.requestHash, order.toOrder(items.findByOrderIds(listOf(saved.orderId))))
    }

    override fun create(draft: OrderDraft, key: UUID, requestHash: String, keyTtl: Duration): Order {
        val order = OrderEntity(
            eventId = draft.session.eventId,
            sessionId = draft.session.sessionId,
            customerId = draft.customer.value,
            customerEmail = draft.customerEmail,
            currency = draft.currency,
            orderedAt = draft.orderedAt,
            status = "PENDING",
        )
        em.persist(order)                                       // IDENTITY: the INSERT runs now and sets order.id
        val orderId = order.id!!
        val venueId = venueOf(draft.session.eventId)

        val rows = draft.items.map {
            OrderItemEntity(
                id = OrderItemId(
                    orderId, draft.session.eventId, draft.session.sessionId, venueId,
                    it.seat.sectorId, it.seat.row, it.seat.number,
                ),
                holderFirstName = it.holder.firstName,
                holderLastName = it.holder.lastName,
                price = it.price,
            )
        }
        rows.forEach { em.persist(it) }
        em.persist(
            IdempotencyKeyEntity(
                id = IdempotencyKeyId(draft.customer.value, key),
                orderId = orderId,
                requestHash = requestHash,
                expiresAt = draft.orderedAt.plus(keyTtl),
            )
        )
        em.flush()                                              // a key already used fails here, inside this call
        return order.toOrder(rows)
    }

    override fun page(customer: CustomerId, after: OrderCursor?, size: Int): List<Order> {
        val limit = Limit.of(size)
        val rows =
            if (after == null) orders.firstPage(customer.value, limit)
            else orders.pageAfter(customer.value, after.orderedAt, after.id, limit)
        if (rows.isEmpty()) return emptyList()                  // no orders: no query for the items
        val itemsByOrder = items.findByOrderIds(rows.map { it.id!! }).groupBy { it.id.orderId }
        return rows.map { it.toOrder(itemsByOrder[it.id!!].orEmpty()) }
    }

    override fun findById(orderId: Long): Order? {
        val entity = orders.findByIdOrNull(orderId) ?: return null
        val itemRows = items.findByOrderIds(listOf(orderId))
        return entity.toOrder(itemRows)
    }

    override fun cancel(customer: CustomerId, orderId: Long, reason: String, now: Instant): Order {
        val entity = orders.findByIdOrNull(orderId) ?: throw NotFound("order $orderId")
        if (entity.customerId != customer.value) throw NotFound("order $orderId")
        if (entity.status != "PAID") throw OrderNotCancellable("order $orderId is not paid (status: ${entity.status})")

        entity.status = "CANCELLED"
        entity.cancelledAt = now
        entity.cancellationReason = reason
        orders.save(entity)

        val itemRows = items.findByOrderIds(listOf(orderId))
        val newVersion = (em.createNativeQuery("SELECT version FROM session WHERE event_id = :eventId AND id = :sessionId")
            .setParameter("eventId", entity.eventId)
            .setParameter("sessionId", entity.sessionId)
            .singleResult as Number).toLong() + 1

        em.createNativeQuery(
            """
            UPDATE session SET version = :newVersion
            WHERE event_id = :eventId AND id = :sessionId
            """
        )
            .setParameter("newVersion", newVersion)
            .setParameter("eventId", entity.eventId)
            .setParameter("sessionId", entity.sessionId)
            .executeUpdate()

        for (item in itemRows) {
            em.createNativeQuery(
                """
                UPDATE session_seat
                SET state = 'FREE', customer_id = NULL, expires_at = NULL, hold_token = NULL
                WHERE event_id = :eventId AND session_id = :sessionId
                  AND venue_id = :venueId AND sector_id = :sectorId AND row = :row AND number = :number
                  AND state = 'SOLD'
                """
            )
                .setParameter("eventId", item.id.eventId)
                .setParameter("sessionId", item.id.sessionId)
                .setParameter("venueId", item.id.venueId)
                .setParameter("sectorId", item.id.sectorId)
                .setParameter("row", item.id.row)
                .setParameter("number", item.id.number)
                .executeUpdate()

            publisher.publishEvent(
                SeatStateChangedEvent(
                    SessionRef(item.id.eventId, item.id.sessionId),
                    SeatRef(item.id.sectorId, item.id.row, item.id.number),
                    SeatStatus.FREE,
                    newVersion,
                )
            )
        }

        em.flush()
        return entity.toOrder(itemRows)
    }

    /** The venue of the event: order_item needs it for its key, the API does not carry it (ADR-0003). */
    private fun venueOf(eventId: Long): Long =
        (em.createNativeQuery("SELECT venue_id FROM event WHERE id = :eventId")
            .setParameter("eventId", eventId)
            .singleResult as Number).toLong()

    private fun OrderEntity.toOrder(itemRows: List<OrderItemEntity>) = Order(
        id = id!!,
        session = SessionRef(eventId, sessionId),
        customer = CustomerId(customerId),
        customerEmail = customerEmail,
        currency = currency,
        orderedAt = orderedAt,
        items = itemRows
            .map { it.toOrderItem() }
            .sortedWith(compareBy({ it.seat.sectorId }, { it.seat.row }, { it.seat.number })),
        state = toState(),
    )

    // The CHECKs of V1 guarantee the columns of each status: no !! can fail on a row of the database
    private fun OrderEntity.toState(): OrderState = when (status) {
        "PENDING" -> OrderState.Pending(payerActionUrl)
        "PAID" -> OrderState.Paid(paidAt!!)
        "FAILED" -> OrderState.Failed(failedAt!!, FailureReason.valueOf(failureReason!!))
        "CANCELLED" -> OrderState.Cancelled(paidAt!!, cancelledAt!!, cancellationReason!!)
        else -> error("unknown order status: $status")
    }

    private fun OrderItemEntity.toOrderItem() = OrderItem(
        seat = SeatRef(id.sectorId, id.row, id.number),
        holder = Holder(holderFirstName, holderLastName),
        price = price,
    )
}