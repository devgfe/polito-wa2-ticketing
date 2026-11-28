//! An in-memory `VenueRepository` for the tests: no database.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use uuid::Uuid;

use super::domain::{
    City, NewSector, NewVenue, Row, Seat, SeatNumber, Sector, SectorId, SectorName, Venue, VenueId,
    VenueName,
};
use super::service::{RepositoryError, VenueRepository, VenueService};

/// Keeps the venues in a list; when `broken`, every call fails like a database that is down.
#[derive(Default)]
pub struct FakeRepo {
    venues: Mutex<Vec<Venue>>,
    broken: bool,
}

impl FakeRepo {
    fn check(&self) -> Result<(), RepositoryError> {
        if self.broken {
            return Err(RepositoryError::Failed("the database is down".to_string()));
        }
        Ok(())
    }
}

#[async_trait]
impl VenueRepository for FakeRepo {
    async fn save(&self, new_venue: NewVenue) -> Result<Venue, RepositoryError> {
        self.check()?;
        let mut venues = self.venues.lock().unwrap();

        // Ids are assigned as the database would: 1, 2, 3... for venues and for sectors.
        let first_sector = venues.iter().map(|v| v.sectors.len()).sum::<usize>() as i64 + 1;
        let sectors = new_venue
            .get_sectors()
            .iter()
            .enumerate()
            .map(|(i, sector)| {
                Sector::new(
                    SectorId::new(first_sector + i as i64),
                    sector.get_name().clone(),
                    sector.get_seats().to_vec(),
                )
            })
            .collect();

        let venue = Venue::new(
            VenueId::new(venues.len() as i64 + 1),
            new_venue.get_name().clone(),
            new_venue.get_city().clone(),
            new_venue.get_time_zone(),
            new_venue.get_owner_id(),
            sectors,
        );
        venues.push(venue.clone());
        Ok(venue)
    }

    async fn find(&self, id: VenueId) -> Result<Option<Venue>, RepositoryError> {
        self.check()?;
        Ok(self
            .venues
            .lock()
            .unwrap()
            .iter()
            .find(|v| v.id == id)
            .cloned())
    }
}

/// A service over an empty repository.
pub fn service() -> VenueService {
    VenueService::new(Arc::new(FakeRepo::default()))
}

/// A service whose repository always fails.
pub fn broken_service() -> VenueService {
    VenueService::new(Arc::new(FakeRepo {
        broken: true,
        ..Default::default()
    }))
}

/// A valid new venue: one sector with seats A1 and A2.
pub fn new_venue() -> NewVenue {
    let seat = |row: &str, number: i64| {
        Seat::new(
            Row::try_from(row.to_string()).unwrap(),
            SeatNumber::try_from(number).unwrap(),
        )
    };
    let stalls = NewSector::new(
        SectorName::try_from("Stalls".to_string()).unwrap(),
        vec![seat("A", 1), seat("A", 2)],
    )
    .unwrap();
    NewVenue::new(
        VenueName::try_from("Teatro Regio".to_string()).unwrap(),
        City::try_from("Torino".to_string()).unwrap(),
        chrono_tz::Europe::Rome,
        Uuid::nil(),
        vec![stalls],
    )
    .unwrap()
}
