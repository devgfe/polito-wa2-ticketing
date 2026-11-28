use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::domain::{Event, EventChanges, EventError, EventId, NewEvent};
use super::domain::{VenueId, VenueSummary};
use super::service::EventRepository;

pub struct PgEventRepository {
    pool: PgPool,
}

impl PgEventRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

// ---------- Righe del DB ----------

/// Riga "piatta" della JOIN event + venue.
#[derive(sqlx::FromRow)]
struct EventRow {
    id: i64,
    organizer_id: Uuid,
    name: String,
    currency: String,
    sale_start: NaiveDateTime,
    published_at: Option<DateTime<Utc>>,
    venue_id: i64,
    venue_name: String,
    venue_city: String,
    venue_time_zone: String,
}

impl EventRow {
    fn into_event(self) -> Event {
        Event {
            id: EventId(self.id),
            venue: VenueSummary {
                id: VenueId(self.venue_id),
                name: self.venue_name,
                city: self.venue_city,
                time_zone: self.venue_time_zone,
            },
            organizer_id: self.organizer_id,
            name: self.name,
            currency: self.currency,
            sale_start: self.sale_start,
            published_at: self.published_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct VenueRow {
    id: i64,
    name: String,
    city: String,
    time_zone: String,
}

// macro per permettere il concat
macro_rules! select_event {
    () => {
        "SELECT e.id, e.organizer_id, e.name, e.currency, e.sale_start, e.published_at,
                v.id AS venue_id, v.name AS venue_name, v.city AS venue_city,
                v.time_zone AS venue_time_zone
         FROM event e
         JOIN venue v ON v.id = e.venue_id"
    };
}

// ---------- Errori ----------

fn storage(err: sqlx::Error) -> EventError {
    tracing::error!(error = %err, "database error");
    EventError::Storage(err.to_string())
}

fn is_foreign_key_violation(err: &sqlx::Error) -> bool {
    matches!(err, sqlx::Error::Database(db) if db.is_foreign_key_violation())
}

// ---------- Implementazione della porta ----------

#[async_trait]
impl EventRepository for PgEventRepository {
    async fn find(&self, id: EventId) -> Result<Option<Event>, EventError> {
        let row: Option<EventRow> = sqlx::query_as(concat!(select_event!(), " WHERE e.id = $1"))
            .bind(id.0)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?;
        Ok(row.map(EventRow::into_event))
    }

    async fn list_published(
        &self,
        city: Option<&str>,
        after: Option<EventId>,
        limit: i64,
    ) -> Result<Vec<Event>, EventError> {
        let sql = concat!(
            select_event!(),
            " WHERE e.published_at IS NOT NULL
                AND ($1::text IS NULL OR v.city = $1)
                AND ($2::bigint IS NULL OR e.id > $2)
              ORDER BY e.id
              LIMIT $3"
        );
        let rows: Vec<EventRow> = sqlx::query_as(sql)
            .bind(city)
            .bind(after.map(|a| a.0))
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?;
        Ok(rows.into_iter().map(EventRow::into_event).collect())
    }

    async fn venue_summary(&self, id: VenueId) -> Result<Option<VenueSummary>, EventError> {
        let row: Option<VenueRow> =
            sqlx::query_as("SELECT id, name, city, time_zone FROM venue WHERE id = $1")
                .bind(id.0)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)?;
        Ok(row.map(|r| VenueSummary {
            id: VenueId(r.id),
            name: r.name,
            city: r.city,
            time_zone: r.time_zone,
        }))
    }

    async fn insert(&self, new: NewEvent) -> Result<Event, EventError> {
        let result: Result<(i64,), sqlx::Error> = sqlx::query_as(
            "INSERT INTO event (venue_id, organizer_id, name, currency, sale_start)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id",
        )
        .bind(new.venue_id.0)
        .bind(new.organizer_id)
        .bind(&new.name)
        .bind(&new.currency)
        .bind(new.sale_start)
        .fetch_one(&self.pool)
        .await;

        let (id,) = match result {
            Ok(row) => row,
            // Il venue è sparito tra il controllo del service e l'INSERT: la FK lo blocca.
            Err(e) if is_foreign_key_violation(&e) => {
                return Err(EventError::UnknownVenue(new.venue_id));
            }
            Err(e) => return Err(storage(e)),
        };

        let id = EventId(id);
        self.find(id).await?.ok_or(EventError::NotFound(id))
    }

    async fn update(&self, id: EventId, changes: &EventChanges) -> Result<Event, EventError> {
        // COALESCE($n, colonna): se il parametro è NULL (campo non inviato) resta il valore attuale.
        let result = sqlx::query(
            "UPDATE event SET
                 name       = COALESCE($2, name),
                 venue_id   = COALESCE($3, venue_id),
                 currency   = COALESCE($4, currency),
                 sale_start = COALESCE($5, sale_start)
             WHERE id = $1",
        )
        .bind(id.0)
        .bind(changes.name.as_deref())
        .bind(changes.venue_id.map(|v| v.0))
        .bind(changes.currency.as_deref())
        .bind(changes.sale_start)
        .execute(&self.pool)
        .await;

        match result {
            Ok(done) if done.rows_affected() == 0 => return Err(EventError::NotFound(id)),
            Ok(_) => {}
            // TODO: con sessioni già create il venue non può cambiare (Foreign Key di session_seat):
            // il service dovrà rifiutarlo prima, con un 409.
            Err(e) if is_foreign_key_violation(&e) && changes.venue_id.is_some() => {
                return Err(EventError::UnknownVenue(changes.venue_id.unwrap()));
            }
            Err(e) => return Err(storage(e)),
        }

        self.find(id).await?.ok_or(EventError::NotFound(id))
    }

    async fn delete(&self, id: EventId) -> Result<(), EventError> {
        // Le Foreign Key non hanno ON DELETE CASCADE: si cancellano prima i figli, dal più "profondo".
        // Tutto in una transazione: o sparisce tutto, o non cambia niente.
        let mut tx = self.pool.begin().await.map_err(storage)?;
        for sql in [
            "DELETE FROM session_seat WHERE event_id = $1",
            "DELETE FROM price_tier WHERE event_id = $1",
            "DELETE FROM session WHERE event_id = $1",
            "DELETE FROM event WHERE id = $1",
        ] {
            sqlx::query(sql)
                .bind(id.0)
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
        }
        tx.commit().await.map_err(storage)
    }
}
