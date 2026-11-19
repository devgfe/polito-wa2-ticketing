# ADR-0001: Service decomposition

## Context
The ticketing platform is split into services that are developed, deployed and scaled separately. The project sets three constraints:

- at least three services, each with its own database schema: no service reads or writes the tables of another one;
- both stacks are used (Kotlin + Spring Boot and Rust + Axum);
- services talk both synchronously (HTTP calls) and asynchronously (messages on a broker).

A boundary between two services is useful only where the data on the two sides can change independently. Data that must change in the same transaction has to stay in the same service; otherwise every operation that touches both sides becomes a distributed transaction.

In our domain:

- the state of a seat (free, held, sold) and the order that buys it must always change together;
- payments depend on an external system (PayPal) that cannot take part in our transactions;
- issuing tickets and sending e-mails happen after the payment and can be delayed without harming the customer.

## Decision
The platform has six services:

| Service | Responsibility | Why it is a separate service |
|---|---|---|
| API gateway | Single entry point for the customer app, the organiser back-office and the gate scanner; first check of the token | Clients see one address, the other services stay internal |
| Catalogue | Venues, events, sessions, price tiers | Mostly reads (browsing), with a load very different from the purchase |
| Inventory & orders | Seat states, orders, sales totals | Seat states and orders must change in the same transaction |
| Payments | Payments and refunds through PayPal | Depends on an external system, with its own failures and retries |
| Ticketing | Tickets, their PDFs, gate scans | Works after the payment; gate validation must stay fast even during a sale |
| Notifications | E-mails and push messages | Purely asynchronous work, triggered by messages |

Each service owns its schema. When a service needs data owned by another one, it calls its API or keeps a read-only copy updated by broker messages (data ownership table in M2).

## Rejected
- **Inventory and orders as two services.** Turning held seats into an order would change two databases at once: every purchase would need a distributed transaction.
- **Ticketing inside Inventory & orders.** PDF generation and gate scans would run in the service that handles the sales peak, competing for the same resources.
- **Fewer, larger services (e.g. one backend).** It would not meet the constraints of at least three services and two stacks.

## Consequences
- `+` The purchase path (Inventory & orders, Payments) scales independently of browsing and of gate scans.
- `+` A failure in ticket issuing or e-mail delivery does not block sales.
- `-` Inventory & orders and Ticketing keep read-only copies of catalogue data: after a change in Catalogue, the copies are updated a moment later (eventual consistency).
- `-` Six services to build, deploy and monitor, and four OpenAPI contracts to keep aligned with the code.
