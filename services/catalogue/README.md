# Catalogue

## Conceptual schema

![Catalogue schema](docs/er-schema-catalogue.png)

The diagram was drawn with [designER](https://designer.designerdb.io/).
The editable source is [`docs/er-schema-catalogue.er`](docs/er-schema-catalogue.er): load it in the tool to change the diagram, then export it again as `docs/er-schema-catalogue.png`.

## Logical schema

Primary keys in **bold**, `*` = nullable.

- venue(**id**, name, city, time_zone, owner_id)
- sector(**venue_id** → venue, **id**, name)
- seat(**(venue_id, sector_id)** → sector, **row**, **number**)
- event(**id**, venue_id → venue, organizer_id, name, sale_start, published_at\*, currency)
- session(**event_id** → event, **id**, starts_at, ends_at, sale_end)
- price_tier(**(event_id, session_id)** → session, **id**, name, price)
- session_seat(**(event_id, session_id)** → session, **(venue_id, sector_id, row, number)** → seat, (event_id, session_id, price_tier_id) → price_tier)