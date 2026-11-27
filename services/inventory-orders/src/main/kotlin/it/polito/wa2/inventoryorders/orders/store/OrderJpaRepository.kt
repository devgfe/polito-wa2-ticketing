package it.polito.wa2.inventoryorders.orders.store

import org.springframework.data.domain.Limit
import org.springframework.data.jpa.repository.JpaRepository
import org.springframework.data.jpa.repository.Query
import java.time.Instant
import java.util.UUID

/** The queries on orders, written in JPQL and run by Spring Data. */
interface OrderJpaRepository : JpaRepository<OrderEntity, Long> {

    /** The first page of the orders of a customer, newest first. */
    @Query(
        """
        SELECT o FROM OrderEntity o
        WHERE o.customerId = :customerId
        ORDER BY o.orderedAt DESC, o.id DESC
        """
    )
    fun firstPage(customerId: UUID, limit: Limit): List<OrderEntity>

    /** The orders older than the cursor (orderedAt, id), newest first: one seek on orders_by_customer. */
    @Query(
        """
        SELECT o FROM OrderEntity o
        WHERE o.customerId = :customerId AND (o.orderedAt, o.id) < (:orderedAt, :id)
        ORDER BY o.orderedAt DESC, o.id DESC
        """
    )
    fun pageAfter(customerId: UUID, orderedAt: Instant, id: Long, limit: Limit): List<OrderEntity>
}

/** The queries on order_item. */
interface OrderItemJpaRepository : JpaRepository<OrderItemEntity, OrderItemId> {

    /** The items of all these orders in one query, instead of one query per order. */
    @Query(
        """
        SELECT i FROM OrderItemEntity i
        WHERE i.id.orderId IN :orderIds
        """
    )
    fun findByOrderIds(orderIds: Collection<Long>): List<OrderItemEntity>
}

/** idempotency_key is only read by primary key: findById of JpaRepository is enough. */
interface IdempotencyKeyJpaRepository : JpaRepository<IdempotencyKeyEntity, IdempotencyKeyId>