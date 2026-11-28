
use chrono::{DateTime, NaiveDateTime, Utc};
use uuid::Uuid;

// ---------- Identificatori ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventId(pub i64);

// ---------- Tipi di venues usati da events ----------
// TODO(merge): VenueId e VenueSummary appartengono a venues.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VenueId(pub i64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VenueSummary {
    pub id: VenueId,
    pub name: String,
    pub city: String,
    pub time_zone: String,
}

// ---------- Evento ----------

/// Un evento salvato. `published_at` è `None` finché è una bozza.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub id: EventId,
    pub venue: VenueSummary,
    pub organizer_id: Uuid,
    pub name: String,
    pub currency: String,
    pub sale_start: NaiveDateTime,
    pub published_at: Option<DateTime<Utc>>,
}

/// I dati per creare un evento: non ha ancora un id, lo assegna il DB.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewEvent {
    pub venue_id: VenueId,
    pub organizer_id: Uuid,
    pub name: String,
    pub currency: String,
    pub sale_start: NaiveDateTime,
}

/// Le modifiche chieste da un PATCH: `None` = campo non toccato.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventChanges {
    pub name: Option<String>,
    pub venue_id: Option<VenueId>,
    pub currency: Option<String>,
    pub sale_start: Option<NaiveDateTime>,
}

// ---------- Errori ----------

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EventError {
    #[error("event {} does not exist", .0.0)]
    NotFound(EventId),
    #[error("venue {} does not exist", .0.0)]
    UnknownVenue(VenueId),
    #[error("only the organizer of the event can change it")]
    Forbidden,
    #[error("the event is published")]
    EventPublished,
    #[error("storage error: {0}")]
    Storage(String),
}

// ---------- Regole ----------

impl Event {
    pub fn is_published(&self) -> bool {
        self.published_at.is_some()
    }

    pub fn is_owned_by(&self, user: Uuid) -> bool {
        self.organizer_id == user
    }

    // Business rule: un evento pubblicato non si cancella.
    pub fn ensure_deletable(&self) -> Result<(), EventError> {
        if self.is_published() {
            return Err(EventError::EventPublished);
        }
        Ok(())
    }

    // Business rule: di un evento pubblicato si può cambiare solo il nome.
    pub fn ensure_changeable(&self, changes: &EventChanges) -> Result<(), EventError> {
        let touches_fixed_fields = changes.venue_id.is_some_and(|v| v != self.venue.id)
            || changes.currency.as_ref().is_some_and(|c| *c != self.currency)
            || changes.sale_start.is_some_and(|s| s != self.sale_start);

        if self.is_published() && touches_fixed_fields {
            return Err(EventError::EventPublished);
        }
        Ok(())
    }
}

// ---------- Test ----------

#[cfg(test)]
mod tests {
    use super::*;

    fn draft() -> Event {
        Event {
            id: EventId(1),
            venue: VenueSummary {
                id: VenueId(3),
                name: "Teatro Regio".into(),
                city: "Torino".into(),
                time_zone: "Europe/Rome".into(),
            },
            organizer_id: Uuid::nil(),
            name: "Aurora Quartet".into(),
            currency: "EUR".into(),
            sale_start: "2026-11-04T18:00:00".parse().unwrap(),
            published_at: None,
        }
    }

    fn published() -> Event {
        Event { published_at: Some(Utc::now()), ..draft() }
    }

    #[test]
    fn a_draft_can_be_deleted() {
        assert_eq!(draft().ensure_deletable(), Ok(()));
    }

    #[test]
    fn a_published_event_cannot_be_deleted() {
        assert_eq!(published().ensure_deletable(), Err(EventError::EventPublished));
    }

    #[test]
    fn the_owner_is_recognised() {
        assert!(draft().is_owned_by(Uuid::nil()));
        assert!(!draft().is_owned_by(Uuid::new_v4()));
    }

    #[test]
    fn a_draft_can_change_everything() {
        let changes = EventChanges { venue_id: Some(VenueId(9)), currency: Some("USD".into()), ..Default::default() };
        assert_eq!(draft().ensure_changeable(&changes), Ok(()));
    }

    #[test]
    fn a_published_event_can_change_only_its_name() {
        let rename = EventChanges { name: Some("New name".into()), ..Default::default() };
        assert_eq!(published().ensure_changeable(&rename), Ok(()));

        let move_venue = EventChanges { venue_id: Some(VenueId(9)), ..Default::default() };
        assert_eq!(published().ensure_changeable(&move_venue), Err(EventError::EventPublished));
    }

    #[test]
    fn sending_the_same_value_is_not_a_change() {
        let same_currency = EventChanges { currency: Some("EUR".into()), ..Default::default() };
        assert_eq!(published().ensure_changeable(&same_currency), Ok(()));
    }
}
