# Catalogue

## Conceptual schema

![Catalogue schema](docs/er-schema-catalogue.png)

The diagram was drawn with [designER](https://designer.designerdb.io/).
The editable source is [`docs/er-schema-catalogue.er`](docs/er-schema-catalogue.er): load it in the tool to change the diagram, then export it again as `docs/er-schema-catalogue.png`.

## Logical schema

Primary keys in **bold**, `*` = nullable, `→ table (columns)` = foreign key.

- venue(**id**, name, city, time_zone, owner_id)
- sector(**venue_id**, **id**, name)
  - venue_id → venue (id)
- seat(**venue_id**, **sector_id**, **row**, **number**)
  - (venue_id, sector_id) → sector (venue_id, id)
- event(**id**, venue_id, organizer_id, name, sale_start, published_at\*, currency)
  - venue_id → venue (id)
- session(**event_id**, **id**, starts_at, ends_at, sale_end)
  - event_id → event (id)
- price_tier(**event_id**, **session_id**, **id**, name, price)
  - (event_id, session_id) → session (event_id, id)
- session_seat(**event_id**, **session_id**, **venue_id**, **sector_id**, **row**, **number**, price_tier_id)
  - (event_id, session_id) → session (event_id, id)
  - (event_id, venue_id) → event (id, venue_id)
  - (venue_id, sector_id, row, number) → seat (venue_id, sector_id, row, number)
  - (event_id, session_id, price_tier_id) → price_tier (event_id, session_id, id)