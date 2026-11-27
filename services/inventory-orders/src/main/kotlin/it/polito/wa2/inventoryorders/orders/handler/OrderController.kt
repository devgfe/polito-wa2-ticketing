package it.polito.wa2.inventoryorders.orders.handler

import it.polito.wa2.inventoryorders.common.CUSTOMER_HEADER
import it.polito.wa2.inventoryorders.common.CustomerId
import it.polito.wa2.inventoryorders.common.DEFAULT_CUSTOMER_ID
import it.polito.wa2.inventoryorders.errors.MalformedRequest
import it.polito.wa2.inventoryorders.orders.domain.OrderCursor
import it.polito.wa2.inventoryorders.orders.service.OrderService
import jakarta.validation.Valid
import org.springframework.http.ResponseEntity
import org.springframework.web.bind.annotation.GetMapping
import org.springframework.web.bind.annotation.PathVariable
import org.springframework.web.bind.annotation.PostMapping
import org.springframework.web.bind.annotation.RequestBody
import org.springframework.web.bind.annotation.RequestHeader
import org.springframework.web.bind.annotation.RequestMapping
import org.springframework.web.bind.annotation.RequestParam
import org.springframework.web.bind.annotation.RestController
import org.springframework.web.util.UriComponentsBuilder
import java.net.URI
import java.nio.charset.StandardCharsets
import java.time.Instant
import java.util.Base64
import java.util.UUID

@RestController
@RequestMapping("/api/v1")
class OrderController(private val orderService: OrderService) {

    @PostMapping("/orders")
    fun create(
        @RequestHeader(name = "Idempotency-Key") idempotencyKey: UUID,
        @RequestHeader(name = CUSTOMER_HEADER, defaultValue = DEFAULT_CUSTOMER_ID) customer: UUID,
        @Valid @RequestBody input: OrderInputDto,
    ): ResponseEntity<OrderResponse> {
        val order = orderService.createOrder(CustomerId(customer), idempotencyKey, input.toDomain())
        val location = UriComponentsBuilder.fromPath("/api/v1/orders/{id}")
            .buildAndExpand(order.id)
            .toUri()
        return ResponseEntity.created(location).body(order.toResponse())
    }

    @GetMapping("/orders")
    fun list(
        @RequestHeader(name = CUSTOMER_HEADER, defaultValue = DEFAULT_CUSTOMER_ID) customer: UUID,
        @RequestParam(required = false) cursor: String?,
        @RequestParam(defaultValue = "20") limit: Int,
    ): OrderPageResponse {
        val parsedLimit = limit.coerceIn(1, 200)
        val orderCursor = cursor?.let { parseCursor(it) }
        val page = orderService.listOrders(CustomerId(customer), orderCursor, parsedLimit)
        val nextCursorToken = page.next?.let { formatCursor(it) }
        return OrderPageResponse(
            items = page.orders.map { it.toResponse() },
            nextCursor = nextCursorToken,
            limit = parsedLimit,
        )
    }

    @GetMapping("/orders/{orderId}")
    fun get(
        @PathVariable orderId: Long,
        @RequestHeader(name = CUSTOMER_HEADER, defaultValue = DEFAULT_CUSTOMER_ID) customer: UUID,
    ): ResponseEntity<OrderResponse> {
        val order = orderService.getOrder(CustomerId(customer), orderId)
        return ResponseEntity.ok(order.toResponse())
    }

    @PostMapping("/orders/{orderId}/cancellation")
    fun cancel(
        @PathVariable orderId: Long,
        @RequestHeader(name = CUSTOMER_HEADER, defaultValue = DEFAULT_CUSTOMER_ID) customer: UUID,
        @Valid @RequestBody input: CancellationInputDto,
    ): ResponseEntity<OrderResponse> {
        val order = orderService.cancelOrder(CustomerId(customer), orderId, input.reason)
        val location = UriComponentsBuilder.fromPath("/api/v1/orders/{id}")
            .buildAndExpand(order.id)
            .toUri()
        return ResponseEntity.created(location).body(order.toResponse())
    }

    private fun formatCursor(cursor: OrderCursor): String {
        val text = "${cursor.orderedAt}:${cursor.id}"
        return Base64.getUrlEncoder().encodeToString(text.toByteArray(StandardCharsets.UTF_8))
    }

    private fun parseCursor(token: String): OrderCursor {
        try {
            val text = String(Base64.getUrlDecoder().decode(token), StandardCharsets.UTF_8)
            val parts = text.split(":")
            return OrderCursor(Instant.parse(parts[0]), parts[1].toLong())
        } catch (ex: Exception) {
            throw MalformedRequest("invalid cursor")
        }
    }
}
