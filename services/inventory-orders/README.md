# Inventory & orders

## Conceptual schema

![Inventory & orders schema](docs/er-schema-inventory-order.png)

The diagram was drawn with [designER](https://designer.designerdb.io/).
The editable source is [`docs/er-schema-inventory-order.er`](docs/er-schema-inventory-order.er): load it in the tool to change the diagram, then export it again as `docs/er-schema-inventory-order.png`.

## Logical schema

Primary keys in **bold**, `*` = nullable.

Read-only copies, updated by broker messages:

- venue(**id**, time_zone)
- sector(**venue_id** → venue, **id**)
- seat(**(venue_id, sector_id)** → sector, **row**, **number**)
- event(**id**, venue_id → venue, organizer_id, sale_start, currency)
- session(**event_id** → event, **id**, starts_at, sale_end, version)
- price_tier(**(event_id, session_id)** → session, **id**, price)

Owned by Inventory & orders:

- session_seat(**(event_id, session_id)** → session, **(venue_id, sector_id, row, number)** → seat, (event_id, session_id, price_tier_id) → price_tier, state, customer_id\*, expires_at\*)
- orders(**id**, customer_id, customer_email, currency, ordered_at, status, payer_action_url\*, paid_at\*, failed_at\*, failure_reason\*, cancelled_at\*, cancellation_reason\*)
- order_item(**order_id** → orders, **(event_id, session_id, venue_id, sector_id, row, number)** → session_seat, holder_first_name, holder_last_name, price)
- idempotency_key(**customer_id**, **key**, order_id → orders, request_hash, expires_at)