use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use super::domain::{Event, EventChanges, EventError, EventId, NewEvent};
use super::domain::{VenueId, VenueSummary};

// ---------- La porta ----------

#[async_trait]
pub trait EventRepository: Send + Sync {
    async fn find(&self, id: EventId) -> Result<Option<Event>, EventError>;

    async fn list_published(
        &self,
        city: Option<&str>,
        after: Option<EventId>,
        limit: i64,
    ) -> Result<Vec<Event>, EventError>;

    async fn venue_summary(&self, id: VenueId) -> Result<Option<VenueSummary>, EventError>;

    async fn insert(&self, event: NewEvent) -> Result<Event, EventError>;
    async fn update(&self, id: EventId, changes: &EventChanges) -> Result<Event, EventError>;
    async fn delete(&self, id: EventId) -> Result<(), EventError>;
}

// ---------- Il service ----------

#[derive(Clone)]
pub struct EventService {
    repo: Arc<dyn EventRepository>,
}

#[derive(Debug)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_after: Option<EventId>,
}

impl EventService {
    pub fn new(repo: Arc<dyn EventRepository>) -> Self {
        Self { repo }
    }

    pub async fn create_event(&self, new: NewEvent) -> Result<Event, EventError> {
        self.ensure_venue_exists(new.venue_id).await?;
        self.repo.insert(new).await
    }

    pub async fn get_event(&self, id: EventId, viewer: Option<Uuid>) -> Result<Event, EventError> {
        let event = self.repo.find(id).await?.ok_or(EventError::NotFound(id))?;

        let can_see = event.is_published() || viewer.is_some_and(|u| event.is_owned_by(u));
        if !can_see {
            return Err(EventError::NotFound(id));
        }
        Ok(event)
    }

    pub async fn list_events(
        &self,
        city: Option<&str>,
        after: Option<EventId>,
        limit: i64,
    ) -> Result<Page<Event>, EventError> {
        let mut items = self.repo.list_published(city, after, limit + 1).await?;

        let next_after = if items.len() as i64 > limit {
            items.truncate(limit as usize);
            items.last().map(|e| e.id)
        } else {
            None
        };
        Ok(Page { items, next_after })
    }

    pub async fn update_event(
        &self,
        id: EventId,
        user: Uuid,
        changes: EventChanges,
    ) -> Result<Event, EventError> {
        let event = self.repo.find(id).await?.ok_or(EventError::NotFound(id))?;
        if !event.is_owned_by(user) {
            return Err(EventError::Forbidden);
        }
        event.ensure_changeable(&changes)?;
        if let Some(venue_id) = changes.venue_id {
            self.ensure_venue_exists(venue_id).await?;
        }
        self.repo.update(id, &changes).await
    }

    pub async fn delete_event(&self, id: EventId, user: Uuid) -> Result<(), EventError> {
        let Some(event) = self.repo.find(id).await? else {
            return Ok(());
        };
        if !event.is_owned_by(user) {
            return Err(EventError::Forbidden);
        }
        event.ensure_deletable()?;
        self.repo.delete(id).await
    }

    async fn ensure_venue_exists(&self, id: VenueId) -> Result<(), EventError> {
        match self.repo.venue_summary(id).await? {
            Some(_) => Ok(()),
            None => Err(EventError::UnknownVenue(id)),
        }
    }
}

// ---------- Test con repository finto ----------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::fake::{event, new_event, service_with, venue};

    // --- create ---

    #[tokio::test]
    async fn create_with_unknown_venue_fails() {
        let service = service_with(vec![]);
        let err = service.create_event(new_event(99, Uuid::new_v4())).await.unwrap_err();
        assert_eq!(err, EventError::UnknownVenue(VenueId(99)));
    }

    #[tokio::test]
    async fn create_returns_a_draft_with_its_venue() {
        let service = service_with(vec![]);
        let event = service.create_event(new_event(3, Uuid::new_v4())).await.unwrap();
        assert!(!event.is_published());
        assert_eq!(event.venue.city, "Torino");
    }

    // --- get ---

    #[tokio::test]
    async fn a_draft_is_hidden_from_others_but_visible_to_its_organizer() {
        let owner = Uuid::new_v4();
        let service = service_with(vec![event(1, owner, false)]);

        assert_eq!(service.get_event(EventId(1), None).await, Err(EventError::NotFound(EventId(1))));
        assert_eq!(
            service.get_event(EventId(1), Some(Uuid::new_v4())).await,
            Err(EventError::NotFound(EventId(1)))
        );
        assert!(service.get_event(EventId(1), Some(owner)).await.is_ok());
    }

    #[tokio::test]
    async fn a_published_event_is_visible_to_anyone() {
        let service = service_with(vec![event(1, Uuid::new_v4(), true)]);
        assert!(service.get_event(EventId(1), None).await.is_ok());
    }

    // --- list ---

    #[tokio::test]
    async fn list_shows_only_published_events_page_by_page() {
        let o = Uuid::new_v4();
        let service = service_with(vec![
            event(1, o, true),
            event(2, o, false), // bozza: non deve comparire
            event(3, o, true),
            event(4, o, true),
        ]);

        let first = service.list_events(None, None, 2).await.unwrap();
        let ids: Vec<i64> = first.items.iter().map(|e| e.id.0).collect();
        assert_eq!(ids, vec![1, 3]);
        assert_eq!(first.next_after, Some(EventId(3)));

        let second = service.list_events(None, first.next_after, 2).await.unwrap();
        let ids: Vec<i64> = second.items.iter().map(|e| e.id.0).collect();
        assert_eq!(ids, vec![4]);
        assert_eq!(second.next_after, None);
    }

    #[tokio::test]
    async fn list_filters_by_city() {
        let o = Uuid::new_v4();
        let mut in_milano = event(2, o, true);
        in_milano.venue = venue(4, "Milano");
        let service = service_with(vec![event(1, o, true), in_milano]);

        let page = service.list_events(Some("Milano"), None, 20).await.unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].id, EventId(2));
    }

    // --- update ---

    #[tokio::test]
    async fn the_organizer_can_rename_a_draft() {
        let owner = Uuid::new_v4();
        let service = service_with(vec![event(1, owner, false)]);
        let changes = EventChanges { name: Some("New name".into()), ..Default::default() };

        let updated = service.update_event(EventId(1), owner, changes).await.unwrap();
        assert_eq!(updated.name, "New name");
    }

    #[tokio::test]
    async fn only_the_organizer_can_update() {
        let service = service_with(vec![event(1, Uuid::new_v4(), false)]);
        let changes = EventChanges { name: Some("x".into()), ..Default::default() };
        let err = service.update_event(EventId(1), Uuid::new_v4(), changes).await.unwrap_err();
        assert_eq!(err, EventError::Forbidden);
    }

    #[tokio::test]
    async fn moving_a_draft_to_an_unknown_venue_fails() {
        let owner = Uuid::new_v4();
        let service = service_with(vec![event(1, owner, false)]);
        let changes = EventChanges { venue_id: Some(VenueId(99)), ..Default::default() };
        let err = service.update_event(EventId(1), owner, changes).await.unwrap_err();
        assert_eq!(err, EventError::UnknownVenue(VenueId(99)));
    }

    #[tokio::test]
    async fn moving_a_published_event_fails() {
        let owner = Uuid::new_v4();
        let service = service_with(vec![event(1, owner, true)]);
        let changes = EventChanges { venue_id: Some(VenueId(4)), ..Default::default() };
        let err = service.update_event(EventId(1), owner, changes).await.unwrap_err();
        assert_eq!(err, EventError::EventPublished);
    }

    #[tokio::test]
    async fn updating_a_missing_event_is_not_found() {
        let service = service_with(vec![]);
        let err = service.update_event(EventId(7), Uuid::new_v4(), EventChanges::default()).await.unwrap_err();
        assert_eq!(err, EventError::NotFound(EventId(7)));
    }

    // --- delete ---

    #[tokio::test]
    async fn deleting_a_published_event_fails() {
        let owner = Uuid::new_v4();
        let service = service_with(vec![event(1, owner, true)]);
        let err = service.delete_event(EventId(1), owner).await.unwrap_err();
        assert_eq!(err, EventError::EventPublished);
    }

    #[tokio::test]
    async fn only_the_organizer_can_delete() {
        let service = service_with(vec![event(1, Uuid::new_v4(), false)]);
        let err = service.delete_event(EventId(1), Uuid::new_v4()).await.unwrap_err();
        assert_eq!(err, EventError::Forbidden);
    }

    #[tokio::test]
    async fn deleting_a_missing_event_is_ok() {
        let service = service_with(vec![]);
        assert_eq!(service.delete_event(EventId(42), Uuid::new_v4()).await, Ok(()));
    }

    #[tokio::test]
    async fn the_organizer_can_delete_a_draft() {
        let owner = Uuid::new_v4();
        let service = service_with(vec![event(1, owner, false)]);
        service.delete_event(EventId(1), owner).await.unwrap();
        assert_eq!(service.get_event(EventId(1), Some(owner)).await, Err(EventError::NotFound(EventId(1))));
    }
}
