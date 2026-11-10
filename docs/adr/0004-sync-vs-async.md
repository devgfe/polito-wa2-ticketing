# ADR-0004: Synchronous payment start, asynchronous after the payment

## Context
Services can talk in two ways:

- **synchronously**: an HTTP call, where the caller waits for the answer. The answer is immediate, but the caller depends on the availability and speed of the called service;
- **asynchronously**: a message on the broker, where the sender does not wait. The services are independent, but the result arrives later.

The project requires at least one synchronous call with an explicit timeout and one asynchronous integration.

During a purchase, the customer's confirmation must return the PayPal link where they pay. Everything that happens after the payment (marking the order as paid, issuing tickets and PDFs, sending the e-mail) is not needed in that response.

## Decision
**Rule.** A call is synchronous only when someone is waiting for its result in order to continue. Everything else is a message on the broker.

### Synchronous calls

| Call | Why it is synchronous |
|---|---|
| Customer app, back-office and gate scanner → API gateway → services | The user is waiting for the answer on the screen |
| Inventory & orders → Payments (`createPayment`) | The response of `createOrder` must contain the PayPal link, and only Payments can create it |

`createPayment` is the only synchronous call between two services, and it has an explicit timeout.

### Timeout of `createPayment`
1. Inventory & orders stops waiting when the timeout expires. The order is already saved and stays PENDING, and the customer gets 503 with a `Retry-After` header, which says after how many seconds to try again.
2. The client sends `createOrder` again with the same Idempotency-Key. Inventory & orders finds the order already saved with that key, so it does not create a second one, and since the order is still PENDING without a payment it calls `createPayment` again.
3. Payments recognises the order. If the first call had actually reached PayPal, it returns the payment already created, otherwise it creates it now. Either way the order has only one payment (ADR-0005).

### Asynchronous messages

| Message | From → to | Why it is asynchronous |
|---|---|---|
| venue, event, session and price tier changes | Catalogue → Inventory & orders, Ticketing | They only update read-only copies, and nobody waits for them |
| payment captured, payment failed | Payments → Inventory & orders | The result comes from PayPal later through a webhook, when no request is waiting for it |
| order paid | Inventory & orders → Ticketing | Issuing tickets and PDFs takes time and must not delay or fail a purchase that is already paid |
| order cancelled | Inventory & orders → Payments, Ticketing, Notifications | The refund, the ticket invalidation and the e-mail can happen after the cancellation is confirmed |
| tickets issued | Ticketing → Notifications | Sending the e-mail must not depend on the e-mail provider being available |

### Reliable messages (outbox)
A service never publishes a message directly on the broker while it changes its data, because the two operations could not succeed or fail together. Instead:

1. In the same database transaction that changes the data (e.g. the order becomes PAID), the service also writes the message in an `outbox` table of its own schema.
2. After the commit, a separate step reads the messages from the `outbox` table, publishes them on the broker and marks them as sent.
3. If the transaction is rolled back, the message disappears with it, so no message announces a change that never happened. If the broker is down, the message stays in the table and is published later, so no message is lost.

## Rejected
- **`createPayment` through the broker.** The client would have to poll until the PayPal link is ready: a slower checkout for no benefit.
- **Synchronous calls to Ticketing and Notifications during the purchase.** PDFs and e-mails would add seconds and failure points; an e-mail provider outage would fail a purchase that is already paid.

## Consequences
- `+` One request returns both the order and the PayPal link.
- `+` Problems in ticket issuing or e-mail delivery do not affect sales: the work is retried later.
- `-` Payments must be available for a customer to start a purchase; timeout and retry limit the damage when it is not.
- `-` An order becomes PAID a moment before its tickets exist: for a few seconds the ticket list is empty, and the PDF endpoint answers 409.
