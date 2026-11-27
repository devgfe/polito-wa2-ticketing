package it.polito.wa2.inventoryorders.config

import jakarta.validation.constraints.NotBlank
import org.springframework.boot.context.properties.ConfigurationProperties
import org.springframework.validation.annotation.Validated
import java.time.Duration

/**
 * Typed, validated configuration of the client of Catalogue.
 */
@Validated
@ConfigurationProperties(prefix = "catalogue")
data class CatalogueProperties(
    @field:NotBlank val baseUrl: String,
    val timeout: Duration,
)
