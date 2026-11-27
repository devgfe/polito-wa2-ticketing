package it.polito.wa2.inventoryorders.orders.store

import jakarta.persistence.Column
import jakarta.persistence.Embeddable
import jakarta.persistence.EmbeddedId
import jakarta.persistence.Entity
import jakarta.persistence.GeneratedValue
import jakarta.persistence.GenerationType
import jakarta.persistence.Id
import jakarta.persistence.Table
import org.hibernate.annotations.JdbcTypeCode
import org.hibernate.type.SqlTypes
import java.time.Instant
import java.util.UUID

/** A row of orders; the columns of the other states stay null while the order is PENDING. */
@Entity
@Table(name = "orders")
class OrderEntity(
    @Id
    @GeneratedValue(strategy = GenerationType.IDENTITY)
    var id: Long? = null,                                   // null until the INSERT
    @Column(name = "event_id")
    var eventId: Long,
    @Column(name = "session_id")
    var sessionId: Long,
    @Column(name = "customer_id")
    var customerId: UUID,
    @Column(name = "customer_email")
    var customerEmail: String,
    @JdbcTypeCode(SqlTypes.CHAR)
    @Column(length = 3)
    var currency: String,
    @Column(name = "ordered_at")
    var orderedAt: Instant,
    var status: String,
    @Column(name = "payer_action_url")
    var payerActionUrl: String? = null,
    @Column(name = "paid_at")
    var paidAt: Instant? = null,
    @Column(name = "failed_at")
    var failedAt: Instant? = null,
    @Column(name = "failure_reason")
    var failureReason: String? = null,
    @Column(name = "cancelled_at")
    var cancelledAt: Instant? = null,
    @Column(name = "cancellation_reason")
    var cancellationReason: String? = null,
) {
    // Identity is the primary key; the hash cannot use the id, which changes from null at the INSERT
    override fun equals(other: Any?) = other is OrderEntity && id != null && id == other.id
    override fun hashCode() = javaClass.hashCode()
}

/** Primary key of order_item: the order and the seat of the session, venue included. */
@Embeddable
data class OrderItemId(
    @Column(name = "order_id")
    var orderId: Long,
    @Column(name = "event_id")
    var eventId: Long,
    @Column(name = "session_id")
    var sessionId: Long,
    @Column(name = "venue_id")
    var venueId: Long,
    @Column(name = "sector_id")
    var sectorId: Long,
    var row: String,
    var number: Int,
)

/** A row of order_item: one seat of an order, with its holder and its price in cents. */
@Entity
@Table(name = "order_item")
class OrderItemEntity(
    @EmbeddedId
    var id: OrderItemId,
    @Column(name = "holder_first_name")
    var holderFirstName: String,
    @Column(name = "holder_last_name")
    var holderLastName: String,
    var price: Long,
) {
    override fun equals(other: Any?) = other is OrderItemEntity && id == other.id
    override fun hashCode() = id.hashCode()
}

/** Primary key of idempotency_key: the pair of customer and key (ADR-0005). */
@Embeddable
data class IdempotencyKeyId(
    @Column(name = "customer_id")
    var customerId: UUID,
    var key: UUID,
)

/**
 * A row of idempotency_key: the Idempotency-Key a customer sent with createOrder,
 * the order it created, and the hash of that request, to tell a retry from a reuse (ADR-0005).
 */
@Entity
@Table(name = "idempotency_key")
class IdempotencyKeyEntity(
    @EmbeddedId
    var id: IdempotencyKeyId,
    @Column(name = "order_id")
    var orderId: Long,
    @Column(name = "request_hash")
    var requestHash: String,
    @Column(name = "expires_at")
    var expiresAt: Instant,
) {
    override fun equals(other: Any?) = other is IdempotencyKeyEntity && id == other.id
    override fun hashCode() = id.hashCode()
}