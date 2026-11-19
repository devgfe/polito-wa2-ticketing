# ADR-0005: Payment flow

## Context
Payments go through PayPal, in sandbox mode. PayPal is an external system: no database transaction can include both our data and a PayPal operation, so each step can fail on its own and must be safe to retry. The requirements are:

- a request sent again (e.g. after a network error) creates neither a second order nor a second charge (NFR3);
- purchases and refunds stay correct even if PayPal is slow, unavailable, or reports results late, twice or out of order (NFR4). PayPal reports results through webhooks, delivered at least once;
- no money is lost (business rule 2): every captured payment belongs to exactly one order and every paid order has exactly one captured payment. When an order is cancelled, its payment is not removed but balanced by a refund, so the money kept always matches the orders that are still valid.

## Decision
- **Order.** The client calls `createOrder` with an `Idempotency-Key` header, a UUID generated once per purchase attempt and reused in all its retries. Inventory & orders saves the key with the order, and the pair of customer and key is unique, so two requests of the same customer with the same key can never create two orders, even when they arrive at the same time, while different customers may happen to use the same key without interfering. When a request arrives with a key that this customer has already used, no new order is created and the response is built from the saved order, in its current state. If the request carries different data from the saved order (other seats, holders or e-mail), the key is being reused for another purchase and the answer is 422. If the saved order is still PENDING without a payment, because the first attempt ended with a timeout, the retry continues it by calling `createPayment`.
- **Payment.** Inventory & orders calls `createPayment`. Payments first records the payment as INITIATED, then asks PayPal to create the PayPal order. If PayPal answers, the payment becomes CREATED and its link is returned. If PayPal is unavailable or does not answer in time, Payments keeps the payment INITIATED and answers 503, which Inventory & orders passes on to the client (ADR-0004). When `createPayment` is called again for the same order and the payment is still INITIATED, Payments contacts PayPal again instead of recording a new payment. An order has at most one payment that is not FAILED, so every retry works on the same payment.
- **Approval.** The customer approves on the PayPal page and returns to the app, which reads the order with `getOrder`.
- **Capture.** PayPal sends the `CHECKOUT.ORDER.APPROVED` webhook; Payments verifies its signature and requests the capture. Payments always checks the current state of the payment first, so a duplicate or late webhook changes nothing.
- **Outcome.** Capture completed → message *payment captured* → Inventory & orders marks the order PAID and its seats SOLD. Payment declined or expired → message *payment failed* → order FAILED, seats FREE.
- **Cancellation.** Within the refund window, `cancelOrder` marks the order CANCELLED and frees its seats; the message *order cancelled* makes Payments request the refund from PayPal. The refund is recorded as a new operation: the original payment is never deleted.
- **Money.** On the wire, an amount is a decimal string with an ISO 4217 code (`{"amount": "45.00", "currency": "EUR"}`); in the database it is an integer number of cents. Floating point is never used.

## Rejected
- **Polling PayPal instead of webhooks.** Slower updates, many useless calls, and PayPal limits the number of requests.
- **Storing a hash of the request and the full response with the key.** The saved order already contains the data of the request and the state needed to build the response, so storing them again would only duplicate them.

## Consequences
- `+` Retries of the client, of Inventory & orders and of PayPal never create a second order, payment or capture.
- `+` The history keeps every payment and refund.
- `-` A retry returns the order in its current state, not a copy of the first response, so a late retry may already see the order as PAID or FAILED.
- `-` The webhook endpoint must be reachable by PayPal on a public HTTPS address, through the API gateway.
- `-` Open point for M4a (payment saga): a customer may approve after the holds expired, so the capture would take money for an order whose seats are free again. The saga must detect this case and refund automatically, or prevent the capture.
