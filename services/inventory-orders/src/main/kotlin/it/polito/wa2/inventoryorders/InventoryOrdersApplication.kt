package it.polito.wa2.inventoryorders

import it.polito.wa2.inventoryorders.config.OrdersProperties
import it.polito.wa2.inventoryorders.config.PaymentsProperties
import it.polito.wa2.inventoryorders.config.SeatsProperties
import org.springframework.boot.autoconfigure.SpringBootApplication
import org.springframework.boot.context.properties.EnableConfigurationProperties
import org.springframework.boot.runApplication

@SpringBootApplication
@EnableConfigurationProperties(PaymentsProperties::class, SeatsProperties::class, OrdersProperties::class)
class InventoryOrdersApplication

fun main(args: Array<String>) {
    runApplication<InventoryOrdersApplication>(*args)
}
