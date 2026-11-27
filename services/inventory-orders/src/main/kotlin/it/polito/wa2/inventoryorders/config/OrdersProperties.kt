package it.polito.wa2.inventoryorders.config

import org.springframework.boot.context.properties.ConfigurationProperties
import org.springframework.validation.annotation.Validated
import java.time.Duration

/** Typed, validated rules of the orders. */
@Validated
@ConfigurationProperties(prefix = "orders")
data class OrdersProperties(
    /** How long an Idempotency-Key protects against a second order (24 hours). */
    val idempotencyKeyTtl: Duration,
)