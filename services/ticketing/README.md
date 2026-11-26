# Ticketing

## Conceptual schema

![Ticketing schema](docs/er-schema-ticketing.png)

The diagram was drawn with [designER](https://designer.designerdb.io/).
The editable source is [`docs/er-schema-ticketing.er`](docs/er-schema-ticketing.er): load it in the tool to change the diagram, then export it again as `docs/er-schema-ticketing.png`.

## Logical schema

Primary keys in **bold**, `*` = nullable, `→ table (columns)` = foreign key.

Read-only copies, updated by broker messages:

- venue(**id**, name, city, time_zone)
- sector(**venue_id**, **id**, name)
  - venue_id → venue (id)
- seat(**venue_id**, **sector_id**, **row**, **number**)
  - (venue_id, sector_id) → sector (venue_id, id)
- event(**id**, venue_id, name)
  - venue_id → venue (id)
- session(**event_id**, **id**, starts_at, ends_at)
  - event_id → event (id)
- session_seat(**event_id**, **session_id**, **venue_id**, **sector_id**, **row**, **number**)
  - (event_id, session_id) → session (event_id, id)
  - (event_id, venue_id) → event (id, venue_id)
  - (venue_id, sector_id, row, number) → seat (venue_id, sector_id, row, number)
- orders(**id**, event_id, session_id, customer_id, customer_email, currency, status)
  - (event_id, session_id) → session (event_id, id)
- order_item(**order_id**, **event_id**, **session_id**, **venue_id**, **sector_id**, **row**, **number**, holder_first_name, holder_last_name, price)
  - (order_id, event_id, session_id) → orders (id, event_id, session_id)
  - (event_id, session_id, venue_id, sector_id, row, number) → session_seat (event_id, session_id, venue_id, sector_id, row, number)

Owned by Ticketing:

- ticket(**id**, order_id, event_id, session_id, venue_id, sector_id, row, number, pdf_key\*, gate_staff_id\*, scanned_at\*)
  - (order_id, event_id, session_id, venue_id, sector_id, row, number) → order_item (order_id, event_id, session_id, venue_id, sector_id, row, number)