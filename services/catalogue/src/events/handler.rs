use axum::{
    Json, Router,
    extract::{
        Path, Query, State,
        rejection::{JsonRejection, PathRejection, QueryRejection},
    },
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};

use super::domain::{Event, EventChanges, EventError, EventId, NewEvent};
use super::domain::{VenueId, VenueSummary};
use super::service::EventService;
use crate::AppState;
use crate::auth::CurrentUser;
use crate::common::{self, Pagination, Violation};
use crate::error::{AppError, FieldCode, FieldError, field};

// ---------- Router ----------

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/events", get(list_events).post(create_event))
        .route(
            "/api/v1/events/{event_id}",
            get(get_event).patch(update_event).delete(delete_event),
        )
}

// ---------- Handler ----------

async fn create_event(
    State(service): State<EventService>,
    user: CurrentUser,
    body: Result<Json<EventInput>, JsonRejection>,
) -> Result<Response, AppError> {
    let organizer = user.require()?;
    let Json(input) = body?;

    let mut errors = Vec::new();
    let name = field(input.name, "name", &mut errors, check_event_name);
    let venue_id = field(input.venue_id, "venueId", &mut errors, parse_venue_id);
    let currency = field(
        input.currency,
        "currency",
        &mut errors,
        common::check_currency,
    );
    let sale_start = field(input.sale_start, "saleStart", &mut errors, |v| {
        common::parse_local(&v)
    });

    let (Some(name), Some(venue_id), Some(currency), Some(sale_start)) =
        (name, venue_id, currency, sale_start)
    else {
        return Err(AppError::Validation(errors));
    };

    let new = NewEvent {
        venue_id,
        organizer_id: organizer,
        name,
        currency,
        sale_start,
    };
    let event = service.create_event(new).await?;

    let location = format!("/api/v1/events/{}", event.id.0);
    Ok((
        StatusCode::CREATED,
        [(header::LOCATION, location)],
        Json(EventDto::from(event)),
    )
        .into_response())
}

async fn get_event(
    State(service): State<EventService>,
    user: CurrentUser,
    path: Result<Path<String>, PathRejection>,
) -> Result<Json<EventDto>, AppError> {
    let id = event_id(path)?;
    let event = service.get_event(id, user.0).await?;
    Ok(Json(EventDto::from(event)))
}

async fn list_events(
    State(service): State<EventService>,
    State(pagination): State<Pagination>,
    query: Result<Query<ListParams>, QueryRejection>,
) -> Result<Json<EventPageDto>, AppError> {
    let Query(params) = query?;
    let limit = pagination.limit(params.limit)?;
    let after = params
        .cursor
        .as_deref()
        .map(common::decode_cursor)
        .transpose()?
        .map(EventId);

    let page = service
        .list_events(params.city.as_deref(), after, limit)
        .await?;
    Ok(Json(EventPageDto {
        items: page.items.into_iter().map(EventSummaryDto::from).collect(),
        next_cursor: page.next_after.map(|id| common::encode_cursor(id.0)),
        limit,
    }))
}

async fn update_event(
    State(service): State<EventService>,
    user: CurrentUser,
    path: Result<Path<String>, PathRejection>,
    body: Result<Json<EventPatch>, JsonRejection>,
) -> Result<Json<EventDto>, AppError> {
    let organizer = user.require()?;
    let id = event_id(path)?;
    let Json(patch) = body?;

    if patch.name.is_none()
        && patch.venue_id.is_none()
        && patch.currency.is_none()
        && patch.sale_start.is_none()
    {
        return Err(AppError::Validation(vec![FieldError::new(
            "",
            FieldCode::TooFewItems,
        )]));
    }

    let mut errors = Vec::new();
    let changes = EventChanges {
        name: optional(patch.name, "name", &mut errors, check_event_name),
        venue_id: optional(patch.venue_id, "venueId", &mut errors, parse_venue_id),
        currency: optional(
            patch.currency,
            "currency",
            &mut errors,
            common::check_currency,
        ),
        sale_start: optional(patch.sale_start, "saleStart", &mut errors, |v| {
            common::parse_local(&v)
        }),
    };
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }

    let event = service.update_event(id, organizer, changes).await?;
    Ok(Json(EventDto::from(event)))
}

async fn delete_event(
    State(service): State<EventService>,
    user: CurrentUser,
    path: Result<Path<String>, PathRejection>,
) -> Result<StatusCode, AppError> {
    let organizer = user.require()?;
    let id = event_id(path)?;
    service.delete_event(id, organizer).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------- EventError → AppError ----------

impl From<EventError> for AppError {
    fn from(e: EventError) -> Self {
        let detail = e.to_string();
        match e {
            EventError::NotFound(_) => AppError::NotFound(detail),
            EventError::UnknownVenue(_) => AppError::UnknownVenue(detail),
            EventError::Forbidden => AppError::Forbidden(detail),
            EventError::EventPublished => AppError::EventPublished(detail),
            // AppError::Internal scrive il messaggio nei log e non lo manda al client.
            EventError::Storage(message) => AppError::Internal(message),
        }
    }
}

// ---------- DTO di input ----------
// Tutti i campi sono Option, anche gli obbligatori: così un campo mancante non è un errore
// di serde (400) ma di validazione (422 validation_failed, code "required").

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EventInput {
    name: Option<String>,
    venue_id: Option<String>,
    currency: Option<String>,
    sale_start: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EventPatch {
    name: Option<String>,
    venue_id: Option<String>,
    currency: Option<String>,
    sale_start: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListParams {
    city: Option<String>,
    cursor: Option<String>,
    limit: Option<i64>,
}

// ---------- Conversioni dei campi ----------

/// EventName nell'OpenAPI: da 1 a 200 caratteri.
fn check_event_name(name: String) -> Result<String, Violation> {
    common::check_length(name, 1, 200)
}

fn parse_venue_id(value: String) -> Result<VenueId, Violation> {
    common::parse_id(&value).map(VenueId)
}

fn optional<V, T>(
    value: Option<V>,
    path: &str,
    errors: &mut Vec<FieldError>,
    convert: impl FnOnce(V) -> Result<T, Violation>,
) -> Option<T> {
    value.and_then(|v| field(Some(v), path, errors, convert))
}

fn event_id(path: Result<Path<String>, PathRejection>) -> Result<EventId, AppError> {
    let Path(raw) = path?;
    common::parse_id(&raw)
        .map(EventId)
        .map_err(|_| AppError::Malformed(format!("'{raw}' is not a valid event id")))
}

// ---------- DTO di output ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VenueSummaryDto {
    id: String,
    name: String,
    city: String,
    time_zone: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EventDto {
    id: String,
    name: String,
    venue: VenueSummaryDto,
    currency: String,
    sale_start: String,
    /// Assente finché l'evento è una bozza.
    #[serde(skip_serializing_if = "Option::is_none")]
    published_at: Option<String>,
    // TODO: SessionSummary quando ci saranno le sessioni.
    sessions: Vec<serde_json::Value>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EventSummaryDto {
    id: String,
    name: String,
    venue: VenueSummaryDto,
    currency: String,
    sale_start: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    published_at: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EventPageDto {
    items: Vec<EventSummaryDto>,
    next_cursor: Option<String>,
    limit: i64,
}

impl From<VenueSummary> for VenueSummaryDto {
    fn from(v: VenueSummary) -> Self {
        Self {
            id: v.id.0.to_string(),
            name: v.name,
            city: v.city,
            time_zone: v.time_zone,
        }
    }
}

impl From<Event> for EventDto {
    fn from(e: Event) -> Self {
        Self {
            id: e.id.0.to_string(),
            name: e.name,
            venue: e.venue.into(),
            currency: e.currency,
            sale_start: common::format_local(e.sale_start),
            published_at: e.published_at.map(common::format_instant),
            sessions: Vec::new(),
        }
    }
}

impl From<Event> for EventSummaryDto {
    fn from(e: Event) -> Self {
        Self {
            id: e.id.0.to_string(),
            name: e.name,
            venue: e.venue.into(),
            currency: e.currency,
            sale_start: common::format_local(e.sale_start),
            published_at: e.published_at.map(common::format_instant),
        }
    }
}

// ---------- Test HTTP (senza database: usano il repository finto) ----------

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::{HeaderMap, Request, StatusCode},
    };
    use serde_json::{Value, json};
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt; // per .oneshot()
    use uuid::Uuid;

    use super::router;
    use crate::AppState;
    use crate::common::Pagination;
    use crate::events::domain::Event;
    use crate::events::fake::{event, service_with};

    use crate::venues::{service::VenueService, store::PgVenueRepository};
    use std::sync::Arc;

    fn app(events: Vec<Event>) -> axum::Router {
        // Pool "pigro": non si collega finché nessuno lo usa.
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://unused@localhost/unused")
            .unwrap();
        // The venue service is never called by these tests: the lazy pool is never used.
        let venues = VenueService::new(Arc::new(PgVenueRepository::new(pool.clone())));
        let state = AppState {
            pool,
            pagination: Pagination {
                default: 20,
                max: 200,
            },
            events: service_with(events),
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

    const VALID: &str = r#"{"name":"Aurora Quartet","venueId":"3","currency":"EUR","saleStart":"2026-11-04T18:00:00"}"#;

    // --- POST ---

    #[tokio::test]
    async fn create_returns_201_with_location_and_the_draft() {
        let (status, headers, body) = call(
            app(vec![]),
            "POST",
            "/api/v1/events",
            Some(Uuid::new_v4()),
            Some(VALID),
        )
        .await;

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(headers["location"], "/api/v1/events/1");
        assert_eq!(body["id"], "1");
        assert_eq!(
            body["venue"],
            json!({"id":"3","name":"Venue 3","city":"Torino","timeZone":"Europe/Rome"})
        );
        assert_eq!(body["saleStart"], "2026-11-04T18:00:00");
        assert!(
            body.get("publishedAt").is_none(),
            "a draft has no publishedAt"
        );
        assert_eq!(body["sessions"], json!([]));
    }

    #[tokio::test]
    async fn create_without_user_is_401_problem() {
        let (status, headers, body) =
            call(app(vec![]), "POST", "/api/v1/events", None, Some(VALID)).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(headers["content-type"], "application/problem+json");
        assert_eq!(body["code"], "unauthorized");
    }

    #[tokio::test]
    async fn create_with_missing_and_invalid_fields_lists_every_error() {
        let (status, _, body) = call(
            app(vec![]),
            "POST",
            "/api/v1/events",
            Some(Uuid::new_v4()),
            Some(r#"{"name":"","currency":"eur"}"#),
        )
        .await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["code"], "validation_failed");
        let errors: Vec<(String, String)> = body["errors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| {
                (
                    e["field"].as_str().unwrap().into(),
                    e["code"].as_str().unwrap().into(),
                )
            })
            .collect();
        assert_eq!(
            errors,
            vec![
                ("name".into(), "too_short".into()),
                ("venueId".into(), "required".into()),
                ("currency".into(), "invalid_format".into()),
                ("saleStart".into(), "required".into()),
            ]
        );
    }

    #[tokio::test]
    async fn create_with_broken_json_is_400() {
        let (status, _, body) = call(
            app(vec![]),
            "POST",
            "/api/v1/events",
            Some(Uuid::new_v4()),
            Some("{oops"),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["code"], "malformed_request");
    }

    #[tokio::test]
    async fn create_with_unknown_venue_is_422_unknown_venue() {
        let body = VALID.replace(r#""venueId":"3""#, r#""venueId":"99""#);
        let (status, _, body) = call(
            app(vec![]),
            "POST",
            "/api/v1/events",
            Some(Uuid::new_v4()),
            Some(&body),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["code"], "unknown_venue");
    }

    // --- GET ---

    #[tokio::test]
    async fn a_draft_is_404_for_anonymous_and_200_for_its_organizer() {
        let owner = Uuid::new_v4();
        let (status, _, body) = call(
            app(vec![event(1, owner, false)]),
            "GET",
            "/api/v1/events/1",
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["code"], "not_found");

        let (status, _, _) = call(
            app(vec![event(1, owner, false)]),
            "GET",
            "/api/v1/events/1",
            Some(owner),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn a_non_numeric_id_is_400() {
        let (status, _, body) = call(app(vec![]), "GET", "/api/v1/events/abc", None, None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["code"], "malformed_request");
    }

    // --- list ---

    #[tokio::test]
    async fn list_pages_with_the_cursor_until_null() {
        let o = Uuid::new_v4();
        let events = vec![event(1, o, true), event(2, o, false), event(3, o, true)];

        let (status, _, page1) = call(
            app(events.clone()),
            "GET",
            "/api/v1/events?limit=1",
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(page1["items"][0]["id"], "1");
        assert_eq!(page1["limit"], 1);
        let cursor = page1["nextCursor"].as_str().unwrap().to_string();

        let uri = format!("/api/v1/events?limit=1&cursor={cursor}");
        let (_, _, page2) = call(app(events), "GET", &uri, None, None).await;
        assert_eq!(page2["items"][0]["id"], "3"); // il 2 è una bozza
        assert_eq!(page2["nextCursor"], Value::Null);
    }

    #[tokio::test]
    async fn list_limit_out_of_range_is_422() {
        let (status, _, body) =
            call(app(vec![]), "GET", "/api/v1/events?limit=500", None, None).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["errors"][0]["field"], "limit");
        assert_eq!(body["errors"][0]["code"], "out_of_range");
    }

    // --- PATCH ---

    #[tokio::test]
    async fn patch_renames_the_event() {
        let owner = Uuid::new_v4();
        let (status, _, body) = call(
            app(vec![event(1, owner, false)]),
            "PATCH",
            "/api/v1/events/1",
            Some(owner),
            Some(r#"{"name":"New"}"#),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["name"], "New");
    }

    #[tokio::test]
    async fn an_empty_patch_is_422() {
        let owner = Uuid::new_v4();
        let (status, _, body) = call(
            app(vec![event(1, owner, false)]),
            "PATCH",
            "/api/v1/events/1",
            Some(owner),
            Some("{}"),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["errors"][0]["code"], "too_few_items");
    }

    #[tokio::test]
    async fn moving_a_published_event_is_409() {
        let owner = Uuid::new_v4();
        let (status, _, body) = call(
            app(vec![event(1, owner, true)]),
            "PATCH",
            "/api/v1/events/1",
            Some(owner),
            Some(r#"{"venueId":"4"}"#),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["code"], "event_published");
    }

    // --- DELETE ---

    #[tokio::test]
    async fn delete_returns_204_even_twice() {
        let owner = Uuid::new_v4();
        let app = app(vec![event(1, owner, false)]);
        let (status, _, _) =
            call(app.clone(), "DELETE", "/api/v1/events/1", Some(owner), None).await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        let (status, _, _) = call(app, "DELETE", "/api/v1/events/1", Some(owner), None).await;
        assert_eq!(status, StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn deleting_someone_elses_event_is_403() {
        let (status, _, body) = call(
            app(vec![event(1, Uuid::new_v4(), false)]),
            "DELETE",
            "/api/v1/events/1",
            Some(Uuid::new_v4()),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["code"], "forbidden");
    }
}
