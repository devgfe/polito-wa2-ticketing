use crate::auth::CurrentUser;
use crate::{
    AppState,
    error::{AppError, FieldCode, FieldError, field},
    venues::{
        domain::{
            City, NewSector, NewVenue, Row, Seat, SeatNumber, Sector, SectorName, Venue, VenueId,
            VenueName, time_zone,
        },
        service::{RepositoryError, ServiceError},
    },
};
use axum::{
    Json, Router,
    extract::{Path, State, rejection::JsonRejection},
    http::{HeaderName, StatusCode, header},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct VenueInput {
    name: Option<String>,
    city: Option<String>,
    time_zone: Option<String>,
    sectors: Option<Vec<SectorInput>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct SectorInput {
    name: Option<String>,
    seats: Option<Vec<SeatInput>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct SeatInput {
    row: Option<String>,
    number: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VenueDto {
    id: String,
    name: String,
    city: String,
    time_zone: String,
    sectors: Vec<SectorDto>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectorDto {
    id: String,
    name: String,
    seats: Vec<SeatDto>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeatDto {
    row: String,
    number: i32,
}

impl From<Seat> for SeatDto {
    fn from(value: Seat) -> Self {
        SeatDto {
            row: value.row.as_str().to_string(),
            number: value.number.get(),
        }
    }
}

impl From<Sector> for SectorDto {
    fn from(value: Sector) -> Self {
        let mut dto_seats: Vec<SeatDto> = vec![];

        for seat in value.seats {
            let seat_dto: SeatDto = seat.into();
            dto_seats.push(seat_dto);
        }
        SectorDto {
            id: value.id.get().to_string(),
            name: value.name.as_str().to_string(),
            seats: dto_seats,
        }
    }
}

impl From<Venue> for VenueDto {
    fn from(value: Venue) -> Self {
        let mut dto_sectors: Vec<SectorDto> = vec![];

        for sector in value.sectors {
            let dto_sector: SectorDto = sector.into();
            dto_sectors.push(dto_sector);
        }
        VenueDto {
            id: value.id.get().to_string(),
            name: value.name.as_str().to_string(),
            city: value.city.as_str().to_string(),
            time_zone: value.time_zone.to_string(),
            sectors: dto_sectors,
        }
    }
}

impl From<ServiceError> for AppError {
    fn from(value: ServiceError) -> Self {
        match value {
            ServiceError::VenueNotFound => {
                AppError::NotFound("The venue does not exist".to_string())
            }
            ServiceError::Repository(repository_error) => match repository_error {
                RepositoryError::Failed(message) => AppError::Internal(message),
            },
        }
    }
}

impl SeatInput {
    fn into_seat(self, path: &str, errors: &mut Vec<FieldError>) -> Option<Seat> {
        let row = field(self.row, &format!("{path}.row"), errors, Row::try_from);
        let number = field(
            self.number,
            &format!("{path}.number"),
            errors,
            SeatNumber::try_from,
        );

        match (row, number) {
            (Some(row), Some(number)) => Some(Seat::new(row, number)),
            _ => None,
        }
    }
}

impl SectorInput {
    fn into_new_sector(self, path: &str, errors: &mut Vec<FieldError>) -> Option<NewSector> {
        let name = field(
            self.name,
            &format!("{path}.name"),
            errors,
            SectorName::try_from,
        );

        // 2. The seats, each with its own position in the request.
        let mut seats = Vec::new();
        let mut seats_valid = true;

        match self.seats {
            None => {
                errors.push(FieldError::new(
                    format!("{path}.seats"),
                    FieldCode::Required,
                ));
                seats_valid = false;
            }
            Some(seat_inputs) => {
                for (index, seat_input) in seat_inputs.into_iter().enumerate() {
                    let seat_path = format!("{path}.seats[{index}]");
                    match seat_input.into_seat(&seat_path, errors) {
                        Some(seat) => seats.push(seat),
                        None => seats_valid = false,
                    }
                }
            }
        }

        // 3. The rules on the sector as a whole are checked only when all its parts are valid.
        let name = name?;
        if !seats_valid {
            return None;
        }

        // 4. At least one seat, no seat twice.
        match NewSector::new(name, seats) {
            Ok(sector) => Some(sector),
            Err(violation) => {
                errors.push(FieldError::new(format!("{path}.seats"), violation.into()));
                None
            }
        }
    }
}

impl VenueInput {
    /// Converts the request into a new venue, or returns every invalid field at once.
    fn into_new_venue(self, owner_id: uuid::Uuid) -> Result<NewVenue, Vec<FieldError>> {
        let mut errors = Vec::new();

        // 1. The fields of the venue.
        let name = field(self.name, "name", &mut errors, VenueName::try_from);
        let city = field(self.city, "city", &mut errors, City::try_from);
        let zone = field(self.time_zone, "timeZone", &mut errors, |text| {
            time_zone(&text)
        });

        // The sectors, each with its own position in the request.
        let mut sectors = Vec::new();
        match self.sectors {
            None => errors.push(FieldError::new("sectors", FieldCode::Required)),
            Some(sector_inputs) => {
                for (index, sector_input) in sector_inputs.into_iter().enumerate() {
                    let sector_path = format!("sectors[{index}]");
                    if let Some(sector) = sector_input.into_new_sector(&sector_path, &mut errors) {
                        sectors.push(sector);
                    }
                }
            }
        }

        // If anything failed, return every error at once.
        let (Some(name), Some(city), Some(zone)) = (name, city, zone) else {
            return Err(errors);
        };
        if !errors.is_empty() {
            return Err(errors);
        }

        // At least one sector.
        NewVenue::new(name, city, zone, owner_id, sectors)
            .map_err(|violation| vec![FieldError::new("sectors", violation.into())])
    }
}

async fn get_venue(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<VenueDto>, AppError> {
    let raw_id = id
        .trim()
        .parse::<i64>()
        .map_err(|_| AppError::Malformed(format!("The id {id} is not a number")))?;

    let venue_id = VenueId::new(raw_id);

    let venue = state.venues.find(venue_id).await?;
    let venue_dto: VenueDto = venue.into();

    Ok(Json(venue_dto))
}

async fn create_venue(
    State(state): State<AppState>,
    user: CurrentUser,
    input: Result<Json<VenueInput>, JsonRejection>,
) -> Result<(StatusCode, [(HeaderName, String); 1], Json<VenueDto>), AppError> {
    let Json(body) = input.map_err(|e| AppError::Malformed(e.body_text()))?;

    let new_venue = match body.into_new_venue(user.require()?) {
        Ok(new_venue) => new_venue,
        Err(e) => return Err(AppError::Validation(e)),
    };
    let created = state.venues.create(new_venue).await?;
    let created_id = created.id.get();

    let url = format!("/api/v1/venues/{created_id}");

    let venue_dto: VenueDto = created.into();

    Ok((
        StatusCode::CREATED,
        [(header::LOCATION, url)],
        Json(venue_dto),
    ))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/venues/{venue_id}", get(get_venue))
        .route("/api/v1/venues", post(create_venue))
}
