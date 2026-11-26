package it.polito.wa2.inventoryorders.errors

import org.slf4j.LoggerFactory
import org.springframework.http.HttpHeaders
import org.springframework.http.HttpStatus
import org.springframework.http.HttpStatusCode
import org.springframework.http.ProblemDetail
import org.springframework.http.ResponseEntity
import org.springframework.validation.FieldError
import org.springframework.web.bind.MethodArgumentNotValidException
import org.springframework.web.bind.annotation.ExceptionHandler
import org.springframework.web.bind.annotation.RestControllerAdvice
import org.springframework.web.context.request.WebRequest
import org.springframework.web.servlet.mvc.method.annotation.ResponseEntityExceptionHandler
import java.net.URI

private const val PROBLEMS_BASE = "https://ticketing.example/problems/"
private const val RETRY_AFTER_SECONDS = 2

/** The single mapping point from failures to RFC 9457 problem documents. */
@RestControllerAdvice
class ApiExceptionHandler : ResponseEntityExceptionHandler() {

    private val log = LoggerFactory.getLogger(ApiExceptionHandler::class.java)

    /** Our failures: one status and one stable code for each class. */
    @ExceptionHandler(AppException::class)
    fun app(ex: AppException): ResponseEntity<Any> {
        val (status, code, title) = when (ex) {
            is MalformedRequest -> Triple(400, "malformed_request", "Malformed request")
            is NotFound -> Triple(404, "not_found", "Not found")
            is SeatTaken -> Triple(409, "seat_taken", "Seat taken")
            is SeatNotHeld -> Triple(409, "seat_not_held", "Seat not held")
            is HoldLimitReached -> Triple(409, "hold_limit_reached", "Hold limit reached")
            is SalesClosed -> Triple(409, "sales_closed", "Sales closed")
            is HoldChanged -> Triple(412, "hold_changed", "Hold changed")
            is ValidationFailed -> Triple(422, "validation_failed", "Validation failed")
            is IdempotencyKeyReused -> Triple(422, "idempotency_key_reused", "Idempotency key reused")
            is PaymentsUnavailable -> Triple(503, "payments_unavailable", "Payments unavailable")
        }

        // Log level by class of failure (L13, "Contention and server-owned failures")
        when (status) {
            in 500..599 -> log.error("{} {}: {}", status, code, ex.detail)
            409, 412 -> log.info("{} {}: {}", status, code, ex.detail)
            else -> log.debug("{} {}: {}", status, code, ex.detail)
        }

        val body = problem(status, code, title, ex.detail)
        if (ex is ValidationFailed) body.setProperty("errors", ex.errors)

        val response = ResponseEntity.status(status)
        if (ex is PaymentsUnavailable) {
            response.header(HttpHeaders.RETRY_AFTER, RETRY_AFTER_SECONDS.toString())   // ADR-0004
        }
        return response.body<Any>(body)
    }

    /** Every error that Spring detects before the controller ends here. */
    override fun handleExceptionInternal(
        ex: Exception,
        body: Any?,
        headers: HttpHeaders,
        statusCode: HttpStatusCode,
        request: WebRequest,
    ): ResponseEntity<Any>? = when {
        ex is MethodArgumentNotValidException -> app(
            ValidationFailed(ex.bindingResult.fieldErrors.map {
                FieldViolation(it.field, violationCode(it), it.defaultMessage ?: "invalid value")
            })
        )
        statusCode.value() == 400 -> app(MalformedRequest("the request is malformed"))
        statusCode.value() == 404 -> app(NotFound("resource")) 
        else -> super.handleExceptionInternal(ex, body, headers, statusCode, request)
    }

    /** Anything else is a defect: full log inside, opaque 500 outside (L13). */
    @ExceptionHandler(Exception::class)
    fun unexpected(ex: Exception): ProblemDetail {
        log.error("unhandled exception", ex)
        return ProblemDetail.forStatus(HttpStatus.INTERNAL_SERVER_ERROR)
    }

    private fun problem(status: Int, code: String, title: String, detail: String): ProblemDetail =
        ProblemDetail.forStatusAndDetail(HttpStatusCode.valueOf(status), detail).apply {
            type = URI.create(PROBLEMS_BASE + code.replace('_', '-'))
            this.title = title
            setProperty("code", code)
        }

    /** Bean Validation constraint → stable code of FieldError (openapi.yaml). */
    private fun violationCode(error: FieldError): String = when (error.code) {
        "Pattern", "Email" -> "invalid_format"
        "Min", "Max" -> "out_of_range"      // the value of a number
        "Size" -> "invalid_size"            // the length of a string or the number of items of a list
        else -> "invalid_format"
    }
}