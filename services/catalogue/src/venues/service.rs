use std::sync::Arc;

use crate::venues::domain::{NewVenue, Venue, VenueId};
use async_trait::async_trait;

#[derive(Debug)]
pub enum RepositoryError {
    Failed(String),
}

#[async_trait]
pub trait VenueRepository: Send + Sync {
    /// creates a Venue (with ID) from a NewVenue (without ID)
    async fn save(&self, new_venue: NewVenue) -> Result<Venue, RepositoryError>;

    /// gets a Venue by ID, or None if it does not exist
    async fn find(&self, id: VenueId) -> Result<Option<Venue>, RepositoryError>;
}

#[derive(Debug)]
pub enum ServiceError {
    VenueNotFound,
    Repository(RepositoryError),
}

impl From<RepositoryError> for ServiceError {
    fn from(error: RepositoryError) -> Self {
        ServiceError::Repository(error)
    }
}

#[derive(Clone)]
pub struct VenueService {
    venues: Arc<dyn VenueRepository>,
}

impl VenueService {
    pub fn new(venues: Arc<dyn VenueRepository>) -> Self {
        VenueService { venues }
    }

    pub async fn create(&self, new_venue: NewVenue) -> Result<Venue, ServiceError> {
        Ok(self.venues.save(new_venue).await?)
    }

    pub async fn find(&self, id: VenueId) -> Result<Venue, ServiceError> {
        match self.venues.find(id).await? {
            Some(venue) => Ok(venue),
            None => Err(ServiceError::VenueNotFound),
        }
    }
}

// ------------------------- SERVICE TESTS ------------------------

/// The use cases over the in-memory repository: no database.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::venues::fake::{broken_service, new_venue, service};

    #[tokio::test]
    async fn a_created_venue_can_be_found_by_its_id() {
        let service = service();
        let created = service.create(new_venue()).await.unwrap();
        assert_eq!(service.find(created.id).await.unwrap(), created);
    }

    #[tokio::test]
    async fn an_unknown_venue_is_not_found() {
        let result = service().find(VenueId::new(42)).await;
        assert!(matches!(result, Err(ServiceError::VenueNotFound)));
    }

    #[tokio::test]
    async fn a_failure_of_the_repository_reaches_the_caller() {
        let service = broken_service();
        let created = service.create(new_venue()).await;
        assert!(matches!(created, Err(ServiceError::Repository(_))));
        let found = service.find(VenueId::new(1)).await;
        assert!(matches!(found, Err(ServiceError::Repository(_))));
    }
}
