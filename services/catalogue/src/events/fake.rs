use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use super::domain::{Event, EventChanges, EventError, EventId, NewEvent};
use super::service::{EventRepository, EventService};
use super::domain::{VenueId, VenueSummary};

pub struct FakeRepo {
    pub venues: Vec<VenueSummary>,
    pub events: Mutex<Vec<Event>>,
}

impl FakeRepo {
    fn venue(&self, id: VenueId) -> Option<VenueSummary> {
        self.venues.iter().find(|v| v.id == id).cloned()
    }
}

#[async_trait]
impl EventRepository for FakeRepo {
    async fn find(&self, id: EventId) -> Result<Option<Event>, EventError> {
        Ok(self.events.lock().unwrap().iter().find(|e| e.id == id).cloned())
    }

    async fn list_published(
        &self,
        city: Option<&str>,
        after: Option<EventId>,
        limit: i64,
    ) -> Result<Vec<Event>, EventError> {
        let mut list: Vec<Event> = self
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.is_published())
            .filter(|e| city.is_none_or(|c| e.venue.city == c))
            .filter(|e| after.is_none_or(|a| e.id.0 > a.0))
            .cloned()
            .collect();
        list.sort_by_key(|e| e.id.0);
        list.truncate(limit as usize);
        Ok(list)
    }

    async fn venue_summary(&self, id: VenueId) -> Result<Option<VenueSummary>, EventError> {
        Ok(self.venue(id))
    }

    async fn insert(&self, new: NewEvent) -> Result<Event, EventError> {
        let venue = self.venue(new.venue_id).ok_or(EventError::UnknownVenue(new.venue_id))?;
        let mut events = self.events.lock().unwrap();
        let event = Event {
            id: EventId(events.len() as i64 + 1),
            venue,
            organizer_id: new.organizer_id,
            name: new.name,
            currency: new.currency,
            sale_start: new.sale_start,
            published_at: None,
        };
        events.push(event.clone());
        Ok(event)
    }

    async fn update(&self, id: EventId, changes: &EventChanges) -> Result<Event, EventError> {
        let new_venue = changes.venue_id.and_then(|v| self.venue(v));
        let mut events = self.events.lock().unwrap();
        let event = events.iter_mut().find(|e| e.id == id).ok_or(EventError::NotFound(id))?;
        if let Some(name) = &changes.name {
            event.name = name.clone();
        }
        if let Some(venue) = new_venue {
            event.venue = venue;
        }
        if let Some(currency) = &changes.currency {
            event.currency = currency.clone();
        }
        if let Some(sale_start) = changes.sale_start {
            event.sale_start = sale_start;
        }
        Ok(event.clone())
    }

    async fn delete(&self, id: EventId) -> Result<(), EventError> {
        self.events.lock().unwrap().retain(|e| e.id != id);
        Ok(())
    }
}

// --- dati di prova ---

pub fn venue(id: i64, city: &str) -> VenueSummary {
    VenueSummary {
        id: VenueId(id),
        name: format!("Venue {id}"),
        city: city.into(),
        time_zone: "Europe/Rome".into(),
    }
}

pub fn event(id: i64, organizer: Uuid, published: bool) -> Event {
    Event {
        id: EventId(id),
        venue: venue(3, "Torino"),
        organizer_id: organizer,
        name: format!("Event {id}"),
        currency: "EUR".into(),
        sale_start: "2026-11-04T18:00:00".parse().unwrap(),
        published_at: published.then(Utc::now),
    }
}

pub fn new_event(venue_id: i64, organizer: Uuid) -> NewEvent {
    NewEvent {
        venue_id: VenueId(venue_id),
        organizer_id: organizer,
        name: "Aurora Quartet".into(),
        currency: "EUR".into(),
        sale_start: "2026-11-04T18:00:00".parse().unwrap(),
    }
}

pub fn service_with(events: Vec<Event>) -> EventService {
    let repo = FakeRepo {
        venues: vec![venue(3, "Torino"), venue(4, "Milano")],
        events: Mutex::new(events),
    };
    EventService::new(Arc::new(repo))
}
