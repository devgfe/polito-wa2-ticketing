package it.polito.wa2.inventoryorders.errors

import it.polito.wa2.inventoryorders.common.SeatRef
import it.polito.wa2.inventoryorders.common.SessionRef

/**
 * Taxonomy of the failures of the service. Services and handlers throw them;
 * ApiExceptionHandler maps each one to a status and a problem document, in one place.
 */
sealed class AppException(val detail: String) : RuntimeException(detail)

// ---- The request is wrong (client-owned) ----

/** The request cannot be read: malformed body, missing header, path value or header of the wrong form. → 400 malformed_request */
class MalformedRequest(detail: String) : AppException(detail)

/** One or more fields are invalid; all of them are reported. → 422 validation_failed */
class ValidationFailed(val errors: List<FieldViolation>) : AppException("the request has invalid fields")

/** The Idempotency-Key was already used by this customer for a different body (ADR-0005). → 422 */
class IdempotencyKeyReused : AppException("the Idempotency-Key was used for a different request")

/** → 404 not_found */
class NotFound(what: String) : AppException("$what not found")

/** → 401 unauthorized */
class Unauthorized(detail: String = "missing or invalid token") : AppException(detail)

/** → 403 forbidden */
class Forbidden(detail: String = "operation not allowed") : AppException(detail)

// ---- The current state does not allow it (business rules) ----

/** Another customer holds or bought the seat (ADR-0003). → 409 seat_taken */
class SeatTaken(seat: SeatRef) : AppException("seat $seat is held or sold")

/** The customer already holds the maximum number of seats (ADR-0003). → 409 hold_limit_reached */
class HoldLimitReached(limit: Int) : AppException("at most $limit seats can be held in a session")

/** Sales of the session are not open, or already closed. → 409 sales_closed */
class SalesClosed(session: SessionRef) : AppException("sales of $session are not open")

/** The seat is held with another token: a newer hold (ADR-0006). → 412 hold_changed */
class HoldChanged(seat: SeatRef) : AppException("seat $seat is held with another token")

/** An order names a seat that the customer does not hold (ADR-0006). → 409 seat_not_held */
class SeatNotHeld(seat: SeatRef) : AppException("seat $seat is not held by the customer")

/** An order cannot be cancelled (e.g. not paid). → 409 order_not_cancellable */
class OrderNotCancellable(detail: String = "order cannot be cancelled") : AppException(detail)

/** Another request with the same Idempotency-Key is still being processed (ADR-0005). → 409 idempotency_key_in_use */
class IdempotencyKeyInUse : AppException("a request with this Idempotency-Key is still in progress: retry it")

// ---- A dependency failed (server-owned) ----

/** createPayment timed out or failed; the order stays PENDING (ADR-0004). → 503 + Retry-After */
class PaymentsUnavailable(detail: String) : AppException(detail)

/** One invalid field, in the errors[] of a 422 (FieldError in openapi.yaml). */
data class FieldViolation(val field: String, val code: String, val detail: String)