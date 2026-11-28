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
