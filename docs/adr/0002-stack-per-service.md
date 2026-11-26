# ADR-0002: Stack per service

## Context
The project requires both stacks, with versions fixed for the whole semester:

- Kotlin 2.3, Spring Boot 4.1, Spring Framework 7, JDK 21/25, Gradle;
- Rust stable (edition 2024), Axum 0.8, Tokio 1.53, SQLx 0.9

Services in different stacks share no code: they cooperate only through OpenAPI contracts and broker messages. So the stack of each service can be chosen by what the service does, as long as the contracts stay precise.

## Decision

| Service | Stack | Database access | Reason |
|---|---|---|---|
| API gateway | Kotlin + Spring Boot | none | It forwards each request to the right service after checking the token, so it spends most of its time waiting for other services, and Spring MVC on virtual threads handles this waiting with plain blocking code |
| Catalogue | Rust + Axum | SQLx | It is public and mostly read, with browsing traffic that grows sharply when sales open and few simple writes, and Axum on Tokio serves many concurrent reads with little memory per request |
| Inventory & orders | Kotlin + Spring Boot | JPA for the orders, JdbcClient for the seat states | It is the transactional core of the purchase, where seat states and orders change inside database transactions that Spring simply declares with `@Transactional`, and it also serves the live seat map, whose many idle connections cost little on virtual threads. Orders are entities saved and read with JPA; every change of a seat state is a single conditional UPDATE (ADR-0003), written in SQL with JdbcClient inside the same transaction |
| Payments | Kotlin + Spring Boot | JPA | It is called synchronously by Inventory & orders, so using the same stack lets both ends of the most critical call of the purchase share conventions and tooling, and its payments and refunds are records written in transactions like the orders |
| Ticketing | Rust + Axum | SQLx | Gate validation must stay fast and predictable (NFR5) even while tickets are being issued, and Rust has no garbage-collector pauses, while PDF generation runs on Tokio's blocking thread pool (`spawn_blocking`) without slowing down the scans |
| Notifications | Kotlin + Spring Boot | none | It is a simple consumer that reads broker messages and calls the e-mail provider, with no database and no heavy computation, so either stack would fit, and Kotlin is chosen because it is the stack used by most services and the simpler one to use for this kind of work |

## Rejected
- **One stack for every service.** It would not meet the two-stack constraint.
- **Inventory & orders in Rust.** Possible, but then the most critical call of the purchase (Inventory & orders → Payments) would cross stacks; keeping both ends in Kotlin lets them share tooling and conventions.
- **JPA for the seat states too.** JPA changes a row in three steps: it loads the entity, changes it in memory and writes it back. Between the read and the write another customer can take the same seat, which is the race that ADR-0003 forbids. The conditional UPDATE would therefore stay SQL even with JPA, and JPA would only add work.

## Consequences
- `+` The whole purchase path is in one stack.
- `+` Each stack is used where it fits: Rust for the high-volume reads and scans, Kotlin for the transactional services.
- `-` Services in different stacks cannot share code, so the same data, such as a seat or a price, is defined once in Kotlin and once in Rust, and the two definitions stay aligned only through the OpenAPI contracts and the message formats, which must be checked in CI.
- `-` The team must review code in both stacks.
- `-` Inventory & orders uses two access styles: entities for the order aggregate, SQL written by hand for the seat states. Both join the same transaction, and the ports hide the difference from the services (ADR-0007).