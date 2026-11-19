# ADR-0006: A hold is the state of a session seat

## Context
A hold reserves a seat for about ten minutes while the customer pays. It can be modelled in two ways:

- as an **object with its own identifier**, created by `POST /holds` and referenced later by the order;
- as **one of the states of the seat** in that session.

The choice also decides what is locked when a customer holds a seat: the larger the locked object, the more customers wait for each other.

In our schema each seat of a session is already one record of the table `session_seat`, in state FREE, HELD or SOLD (ADR-0003).

## Decision
- **A hold is the HELD state of a `session_seat` record**, with no identifier of its own.
- **API:** `PUT /holds/{sessionSeatId}` holds the seat, `DELETE` on the same path releases it. `sessionSeatId` identifies the seat of a session through the same six attributes that form the key of its `session_seat` record (venue, event, session, sector, row and number). Both are idempotent: a PUT on a seat the caller already holds returns 200 with the same hold, and a DELETE on a seat the caller does not hold returns 204. A client can therefore repeat either request after a network error.
- **A basket of up to four seats is up to four PUTs.** If one seat is taken, only that request fails with 409 and the customer picks another seat; the others stay held.
- **The order lists its seats by `sessionSeatId`**; Inventory & orders checks that they are held by the caller.

## Rejected
- **Hold as a resource with its own id (`POST /holds`).** It adds an identity and a table that our schema does not need.
- **One request for the whole basket (all or none).** It would lock several records together, and one taken seat would block the other three.

## Consequences
- `+` A hold locks one record: customers choosing different seats never wait for each other.
- `+` Retrying PUT and DELETE is safe.
- `+` The seat map is a read of one table.
- `-` Since a hold has no id, a very late repeated DELETE could release a hold that the same customer took again on the same seat.
- `-` Each seat expires on its own: an order can be paid only before its earliest hold expires.
