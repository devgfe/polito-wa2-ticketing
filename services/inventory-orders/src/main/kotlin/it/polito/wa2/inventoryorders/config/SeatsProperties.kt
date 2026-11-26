package it.polito.wa2.inventoryorders.config

import jakarta.validation.constraints.Max
import jakarta.validation.constraints.Min
import org.springframework.boot.context.properties.ConfigurationProperties
import org.springframework.validation.annotation.Validated
import java.time.Duration

/** Typed, validated rules of the holds. */
@Validated
@ConfigurationProperties(prefix = "seats")
data class SeatsProperties(
    /** At most 4: an order cannot have more seats (maxItems in openapi.yaml). */
    @field:Min(1) @field:Max(4) val maxHoldsPerSession: Int,
    /** About ten minutes. */
    val holdTtl: Duration,
)