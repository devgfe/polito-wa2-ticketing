# ADR-0003: Overselling prevention

## Context
The same seat must never be sold twice (NFR1). This is hard because, when sales open, many customers try to take the same seats at the same moment, and each service runs in several replicas.

The obvious approach, "read the seat; if it is free, mark it as held", is wrong: two requests can both read "free" before either writes, and both customers believe they got the seat. A lock inside one process does not help, because the other replicas do not see it. The decision must therefore be taken by the database, the only component shared by all replicas.

## Decision
- Inventory & orders keeps one record per seat and session (table `session_seat`), in state FREE, HELD or SOLD; a held record also stores the customer and the expiry time.
- Every state change is a single conditional UPDATE, which checks the current state and writes the new one in one atomic step. Holding a seat:

  ```sql
  UPDATE session_seat
  SET state = 'HELD', customer_id = $customer, expires_at = now() + interval '10 minutes'
  WHERE venue_id = $venue AND event_id = $event AND session_id = $session
    AND sector_id = $sector AND row = $row AND number = $number
    AND (state = 'FREE' OR (state = 'HELD' AND expires_at < now()));
  ```

  The seat is identified by its key in the schema (venue, event, session, sector, row and number), the same six attributes carried by the `sessionSeatId` of the API, and `interval '10 minutes'` sets the expiry ten minutes after the hold. If the UPDATE changes no record, the seat was taken by someone else: the API answers 409.
- HELD → SOLD happens when the payment is captured, only for the seats of that order held by that customer, in the same transaction that marks the order as PAID.
- Holds expire after about ten minutes without any request. An expired hold is always treated as free, as the hold UPDATE above shows, even if no explicit release has happened yet.

## Rejected
- **Check in the application, then write.** Two requests can both pass the check, as described above.
- **SERIALIZABLE isolation for every purchase.** The database runs the transactions as if they happened one after another, and when two of them touch the same seat at the same time it aborts one with an error, so the application must retry it. This is correct, but when sales open many customers compete for the same seats, so many transactions are aborted and retried, and every retry adds load to the database exactly when it is busiest. With the conditional UPDATE, the losing request simply changes no record and gets an immediate 409, without any retry.
- **One lock on the whole session or event.** Before changing any seat, a transaction would lock the database record of the session (or of the event), so only one customer at a time could hold seats in it while all the others wait. This is correct, but when sales open thousands of customers of the same session would queue behind each other even when they choose different seats, and the waiting time would grow with the crowd. With the conditional UPDATE, only the customers who want the same seat compete with each other.

## Consequences
- `+` Correct with any number of replicas: the database is the only judge.
- `+` A customer who loses the race gets an immediate 409 and picks another seat.
- `-` The limit of four seats per customer involves several records, so a single UPDATE cannot enforce it: it is checked when holding (two parallel requests of the same customer may exceed it) and guaranteed when ordering (an order has at most four seats).
- `-` The database is the busiest component when sales open: the connection pool is sized on its capacity, not on the number of incoming requests.
