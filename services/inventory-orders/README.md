# Inventory & orders

## Conceptual schema

![Inventory & orders schema](docs/er-schema-inventory-order.png)

The diagram was drawn with [designER](https://designer.designerdb.io/).
The editable source is [`docs/er-schema-inventory-order.er`](docs/er-schema-inventory-order.er): load it in the tool to change the diagram, then export it again as `docs/er-schema-inventory-order.png`.

## Logical schema

Primary keys in **bold**, `*` = nullable, `→ table (columns)` = foreign key.

Read-only copies, updated by broker messages:

- venue(**id**, time_zone)
- sector(**venue_id**, **id**)
  - venue_id → venue (id)
- seat(**venue_id**, **sector_id**, **row**, **number**)
  - (venue_id, sector_id) → sector (venue_id, id)
- event(**id**, venue_id, organizer_id, sale_start, currency)
  - venue_id → venue (id)
- session(**event_id**, **id**, starts_at, sale_end, version)
  - event_id → event (id)
- price_tier(**event_id**, **session_id**, **id**, price)
  - (event_id, session_id) → session (event_id, id)

Owned by Inventory & orders:

- session_seat(**event_id**, **session_id**, **venue_id**, **sector_id**, **row**, **number**, price_tier_id, state, customer_id\*, expires_at\*, hold_token\*)
  - (event_id, session_id) → session (event_id, id)
  - (event_id, venue_id) → event (id, venue_id)
  - (venue_id, sector_id, row, number) → seat (venue_id, sector_id, row, number)
  - (event_id, session_id, price_tier_id) → price_tier (event_id, session_id, id)
- orders(**id**, event_id, session_id, customer_id, customer_email, currency, ordered_at, status, payer_action_url\*, paid_at\*, failed_at\*, failure_reason\*, cancelled_at\*, cancellation_reason\*)
  - (event_id, session_id) → session (event_id, id)
- order_item(**order_id**, **event_id**, **session_id**, **venue_id**, **sector_id**, **row**, **number**, holder_first_name, holder_last_name, price)
  - (order_id, event_id, session_id) → orders (id, event_id, session_id)
  - (event_id, session_id, venue_id, sector_id, row, number) → session_seat (event_id, session_id, venue_id, sector_id, row, number)
- idempotency_key(**customer_id**, **key**, order_id, request_hash, expires_at)
  - order_id → orders (id)