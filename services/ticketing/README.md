# Ticketing

## Conceptual schema

![Ticketing schema](docs/er-schema-ticketing.png)

The diagram was drawn with [designER](https://designer.designerdb.io/).
The editable source is [`docs/er-schema-ticketing.er`](docs/er-schema-ticketing.er): load it in the tool to change the diagram, then export it again as `docs/er-schema-ticketing.png`.

## Logical schema

Primary keys in **bold**, `*` = nullable.

Read-only copies, updated by broker messages:

- venue(**id**, name, city, time_zone)
- sector(**venue_id** → venue, **id**, name)
- seat(**(venue_id, sector_id)** → sector, **row**, **number**)
- event(**id**, venue_id → venue, name)
- session(**event_id** → event, **id**, starts_at, ends_at)
- session_seat(**(event_id, session_id)** → session, **(venue_id, sector_id, row, number)** → seat)
- orders(**id**, customer_id, customer_email, currency, status)
- order_item(**order_id** → orders, **(event_id, session_id, venue_id, sector_id, row, number)** → session_seat, holder_first_name, holder_last_name, price)

Owned by Ticketing:

- ticket(**id**, (order_id, event_id, session_id, venue_id, sector_id, row, number) → order_item, pdf_key\*, gate_staff_id\*, scanned_at\*)