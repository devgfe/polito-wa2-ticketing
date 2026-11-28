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

// ------------------------- HANDLER TESTS ------------------------

/// The HTTP contract of the venues (status, headers, body), over the in-memory repository.
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{
        body::{Body, to_bytes},
        http::{HeaderMap, Request, StatusCode},
    };
    use serde_json::{Value, json};
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt; // for .oneshot()
    use uuid::Uuid;

    use super::router;
    use crate::AppState;
    use crate::common::Pagination;
    use crate::events::{service::EventService, store::PgEventRepository};
    use crate::venues::{fake, service::VenueService};

    fn app(venues: VenueService) -> axum::Router {
        // Lazy pool: it never connects, because nothing in these tests uses it.
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://unused@localhost/unused")
            .unwrap();
        // The event service is never called by these tests.
        let events = EventService::new(Arc::new(PgEventRepository::new(pool.clone())));
        let state = AppState {
            pool,
            pagination: Pagination {
                default: 20,
                max: 200,
            },
            events,
            venues,
        };
        router().with_state(state)
    }

    async fn call(
        app: axum::Router,
        method: &str,
        uri: &str,
        user: Option<Uuid>,
        body: Option<&str>,
    ) -> (StatusCode, HeaderMap, Value) {
        let mut req = Request::builder().method(method).uri(uri);
        if let Some(user) = user {
            req = req.header("x-user-id", user.to_string());
        }
        let req = match body {
            Some(b) => req
                .header("content-type", "application/json")
                .body(Body::from(b.to_string())),
            None => req.body(Body::empty()),
        }
        .unwrap();

        let res = app.oneshot(req).await.unwrap();
        let status = res.status();
        let headers = res.headers().clone();
        let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let json = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        };
        (status, headers, json)
    }

    fn user() -> Option<Uuid> {
        Some(Uuid::new_v4())
    }

    /// The `errors` of a problem as (field, code) pairs, to compare them in one assert.
    fn field_errors(problem: &Value) -> Vec<(String, String)> {
        problem["errors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| {
                let field = e["field"].as_str().unwrap().to_string();
                let code = e["code"].as_str().unwrap().to_string();
                (field, code)
            })
            .collect()
    }

    fn pairs<const N: usize>(expected: [(&str, &str); N]) -> Vec<(String, String)> {
        expected
            .iter()
            .map(|(field, code)| (field.to_string(), code.to_string()))
            .collect()
    }

    const VALID: &str = r#"{"name":"Teatro Regio","city":"Torino","timeZone":"Europe/Rome",
        "sectors":[{"name":"Stalls","seats":[{"row":"A","number":1},{"row":"A","number":2}]}]}"#;

    // --- POST, then GET ---

    #[tokio::test]
    async fn a_created_venue_has_a_location_and_reads_back_the_same() {
        let app = app(fake::service());

        let (status, headers, created) =
            call(app.clone(), "POST", "/api/v1/venues", user(), Some(VALID)).await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(headers["location"], "/api/v1/venues/1");
        assert_eq!(
            created,
            json!({
                "id": "1", "name": "Teatro Regio", "city": "Torino", "timeZone": "Europe/Rome",
                "sectors": [{
                    "id": "1", "name": "Stalls",
                    "seats": [{ "row": "A", "number": 1 }, { "row": "A", "number": 2 }]
                }]
            })
        );

        let (status, _, read) = call(app, "GET", "/api/v1/venues/1", None, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(read, created);
    }

    // --- Validation: 422 ---

    #[tokio::test]
    async fn every_invalid_field_is_reported_at_once_with_its_path() {
        // City empty, time zone missing, a seat numbered 0, a seat given twice.
        let body = r#"{"name":"Teatro Regio","city":"","sectors":[
            {"name":"Stalls","seats":[{"row":"A","number":1},{"row":"A","number":0}]},
            {"name":"Gallery","seats":[{"row":"B","number":1},{"row":"B","number":1}]}]}"#;

        let (status, headers, problem) = call(
            app(fake::service()),
            "POST",
            "/api/v1/venues",
            user(),
            Some(body),
        )
        .await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(headers["content-type"], "application/problem+json");
        assert_eq!(problem["code"], "validation_failed");
        assert_eq!(
            field_errors(&problem),
            pairs([
                ("city", "too_short"),
                ("timeZone", "required"),
                ("sectors[0].seats[1].number", "out_of_range"),
                ("sectors[1].seats", "not_unique"),
            ])
        );
    }

    #[tokio::test]
    async fn each_rule_on_the_venue_as_a_whole_is_checked() {
        let seat = r#"[{"row":"A","number":1}]"#;
        let cases = [
            (
                format!(r#"{{"name":"T","city":"C","timeZone":"Roma","sectors":[{{"name":"S","seats":{seat}}}]}}"#),
                "timeZone",
                "invalid_format",
            ),
            (
                r#"{"name":"T","city":"C","timeZone":"Europe/Rome","sectors":[]}"#.to_string(),
                "sectors",
                "too_few_items",
            ),
            (
                r#"{"name":"T","city":"C","timeZone":"Europe/Rome","sectors":[{"name":"S","seats":[]}]}"#
                    .to_string(),
                "sectors[0].seats",
                "too_few_items",
            ),
        ];
        for (body, field, code) in cases {
            let (status, _, problem) = call(
                app(fake::service()),
                "POST",
                "/api/v1/venues",
                user(),
                Some(&body),
            )
            .await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
            assert_eq!(field_errors(&problem), pairs([(field, code)]), "{body}");
        }
    }

    // --- Other errors of the POST ---

    #[tokio::test]
    async fn a_body_that_is_not_the_expected_json_is_a_400() {
        // Not JSON, an unknown field, a field of the wrong type.
        for body in ["{oops", r#"{"name":"T","capacity":10}"#, r#"{"name":5}"#] {
            let (status, _, problem) = call(
                app(fake::service()),
                "POST",
                "/api/v1/venues",
                user(),
                Some(body),
            )
            .await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
            assert_eq!(problem["code"], "malformed_request", "{body}");
        }
    }

    #[tokio::test]
    async fn creating_a_venue_needs_a_user() {
        let (status, _, problem) = call(
            app(fake::service()),
            "POST",
            "/api/v1/venues",
            None,
            Some(VALID),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(problem["code"], "unauthorized");
    }

    #[tokio::test]
    async fn a_failure_of_the_database_is_a_500_that_hides_its_detail() {
        let (status, _, problem) = call(
            app(fake::broken_service()),
            "POST",
            "/api/v1/venues",
            user(),
            Some(VALID),
        )
        .await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(problem["code"], "internal_error");
        assert!(!problem.to_string().contains("the database is down"));
    }

    // --- GET errors ---

    #[tokio::test]
    async fn reading_an_unknown_venue_is_a_404_and_a_non_numeric_id_a_400() {
        let (status, _, problem) =
            call(app(fake::service()), "GET", "/api/v1/venues/42", None, None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(problem["code"], "not_found");

        let (status, _, problem) = call(
            app(fake::service()),
            "GET",
            "/api/v1/venues/abc",
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(problem["code"], "malformed_request");
    }
}
