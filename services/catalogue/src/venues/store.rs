use crate::common::Violation;
use crate::venues::domain::{
    City, NewVenue, Row, Seat, SeatNumber, Sector, SectorId, SectorName, Venue, VenueId, VenueName,
    time_zone,
};
use crate::venues::service::{RepositoryError, VenueRepository};
use async_trait::async_trait;
use sqlx::{FromRow, PgPool};
use std::collections::HashMap;

impl From<sqlx::Error> for RepositoryError {
    fn from(error: sqlx::Error) -> Self {
        Self::Failed(error.to_string())
    }
}

impl From<Violation> for RepositoryError {
    fn from(error: Violation) -> Self {
        RepositoryError::Failed(format!("invalid data in the database: {error:?}"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
struct VenueRow {
    id: i64,
    name: String,
    city: String,
    time_zone: String,
    owner_id: uuid::Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
struct SectorRow {
    venue_id: i64,
    id: i64,
    name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
struct SeatRow {
    venue_id: i64,
    sector_id: i64,
    row: String,
    number: i32,
}

impl TryFrom<SeatRow> for Seat {
    type Error = RepositoryError;
    fn try_from(value: SeatRow) -> Result<Self, Self::Error> {
        let row = Row::try_from(value.row)?;
        let number = SeatNumber::try_from(i64::from(value.number))?;
        Ok(Seat::new(row, number))
    }
}

pub struct PgVenueRepository {
    pool: PgPool,
}

impl PgVenueRepository {
    pub fn new(pool: PgPool) -> PgVenueRepository {
        PgVenueRepository { pool }
    }
}

#[async_trait]
impl VenueRepository for PgVenueRepository {
    async fn save(&self, new_venue: NewVenue) -> Result<Venue, RepositoryError> {
        // Everything in one transaction: either the whole venue is saved, or nothing.
        let mut tx = self.pool.begin().await?;

        // 1. The venue: the database generates its id and returns it.
        let venue_id: i64 = sqlx::query_scalar(
            "INSERT INTO venue (name, city, time_zone, owner_id)
         VALUES ($1, $2, $3, $4)
         RETURNING id",
        )
        .bind(new_venue.get_name().as_str())
        .bind(new_venue.get_city().as_str())
        .bind(new_venue.get_time_zone().name())
        .bind(new_venue.get_owner_id())
        .fetch_one(&mut *tx)
        .await?;

        let mut sectors: Vec<Sector> = Vec::with_capacity(new_venue.get_sectors().len());

        for new_sector in new_venue.get_sectors() {
            let sector_id: i64 = sqlx::query_scalar(
                "INSERT INTO sector(venue_id, name)
                VALUES($1,$2)
                RETURNING id",
            )
            .bind(venue_id)
            .bind(new_sector.get_name().as_str())
            .fetch_one(&mut *tx)
            .await?;

            for new_seat in new_sector.get_seats() {
                sqlx::query(
                    r#"INSERT INTO seat (venue_id, sector_id, "row", number)
                    VALUES ($1,$2,$3,$4)"#,
                )
                .bind(venue_id)
                .bind(sector_id)
                .bind(new_seat.row.as_str())
                .bind(new_seat.number.get())
                .execute(&mut *tx)
                .await?;
            }

            sectors.push(Sector::new(
                SectorId::new(sector_id),
                new_sector.get_name().clone(),
                new_sector.get_seats().to_vec(),
            ));
        }

        tx.commit().await?;

        Ok(Venue::new(
            VenueId::new(venue_id),
            new_venue.get_name().clone(),
            new_venue.get_city().clone(),
            new_venue.get_time_zone(),
            new_venue.get_owner_id(),
            sectors,
        ))
    }

    async fn find(&self, id: VenueId) -> Result<Option<Venue>, RepositoryError> {
        let Some(venue_row) = sqlx::query_as::<_, VenueRow>(
            "SELECT id, name, city, time_zone, owner_id FROM venue WHERE id = $1",
        )
        .bind(id.get())
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(None);
        };

        // 2. All the seats of the venue, grouped by sector.
        let seat_rows = sqlx::query_as::<_, SeatRow>(
            r#"SELECT venue_id, sector_id, "row", number FROM seat
               WHERE venue_id = $1 ORDER BY sector_id, "row", number"#,
        )
        .bind(id.get())
        .fetch_all(&self.pool)
        .await?;

        let mut seats_by_sector: HashMap<i64, Vec<Seat>> = HashMap::new();
        for seat_row in seat_rows {
            let sector_id = seat_row.sector_id;
            let seat = Seat::try_from(seat_row)?;
            seats_by_sector.entry(sector_id).or_default().push(seat);
        }

        // 3. The sectors, each with its own seats taken from the groups.
        let sector_rows = sqlx::query_as::<_, SectorRow>(
            "SELECT venue_id, id, name FROM sector WHERE venue_id = $1 ORDER BY id",
        )
        .bind(id.get())
        .fetch_all(&self.pool)
        .await?;

        let mut sectors = Vec::with_capacity(sector_rows.len());
        for sector_row in sector_rows {
            let seats = seats_by_sector.remove(&sector_row.id).unwrap_or_default();
            sectors.push(Sector::new(
                SectorId::new(sector_row.id),
                SectorName::try_from(sector_row.name)?,
                seats,
            ));
        }

        // 4. The venue, with its sectors.
        Ok(Some(Venue::new(
            VenueId::new(venue_row.id),
            VenueName::try_from(venue_row.name)?,
            City::try_from(venue_row.city)?,
            time_zone(&venue_row.time_zone)?,
            venue_row.owner_id,
            sectors,
        )))
    }
}
