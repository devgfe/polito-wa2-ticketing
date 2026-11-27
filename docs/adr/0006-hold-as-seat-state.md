# ADR-0006: A hold is the state of a session seat

## Context
A hold reserves a seat for about ten minutes while the customer pays. It can be modelled in two ways:

- as an **object with its own identifier**, created by `POST /holds` and referenced later by the order;
- as **one of the states of the seat** in that session.

The choice also decides what is locked when a customer holds a seat: the larger the locked object, the more customers wait for each other.

In our schema each seat of a session is already one record of the table `session_seat`, in state FREE, HELD or SOLD (ADR-0003).

## Decision
- **A hold is the HELD state of a `session_seat` record**, with no identifier of its own. Every hold gets a new token, so two holds of the same seat at different moments can be told apart.
- **API:** the hold is a sub-resource of the seat of a session, `/events/{eventId}/sessions/{sessionId}/seats/{sectorId}/{row}/{number}/hold`. `POST` on it holds the seat, with no body: it answers 201 with `Location` and `ETag` (the token) when the seat is held now, and 200 with the same hold and token when the caller already holds it, without extending its expiry, so a client can repeat the request after a network error. `GET` reads the hold. `DELETE` requires `If-Match` with the token: it releases the seat and answers 204 if the seat is still held with that token, or no longer held at all; it answers 412 if the seat is held with another token, and 400 if `If-Match` is missing.
- **A basket of up to four seats is up to four POSTs.** If one seat is taken, only that request fails with 409 and the customer picks another seat; the others stay held.
- **The order names its session once**, as a `SessionRef` (`eventId`, `sessionId`), **and lists its seats as `SeatRef`**; Inventory & orders checks that they are held by the caller.

## Rejected
- **Hold as a resource with its own id, in a table of its own.** It would also protect against a late DELETE, but it changes how overselling is prevented (ADR-0003) and makes the seat map read two tables; the token gives the same protection with one column.
- **`If-Match` optional on DELETE.** Clients that do not send it would get no protection from a late DELETE.
- **One request for the whole basket (all or none).** It would lock several records together, and one taken seat would block the other three.

## Consequences
- `+` A hold locks one record: customers choosing different seats never wait for each other.
- `+` Repeating the POST is safe, because the seat itself is the natural key of the hold; DELETE is idempotent, since repeating it with the same token never changes more than the first time.
- `+` A late repeated DELETE carries the `If-Match` of the old hold, so it cannot release a newer hold of the same seat: it gets 412.
- `+` The seat map is a read of one table.
- `-` Every client must keep the token of its holds to release them; a client that lost it reads it again with `GET` on the hold or with `GET /holds`.
- `-` Each seat expires on its own: an order can be paid only before its earliest hold expires.