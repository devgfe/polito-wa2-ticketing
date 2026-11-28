use axum::{
    Json,
    extract::rejection::{JsonRejection, PathRejection, QueryRejection},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::common::Violation;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldCode {
    Required,
    InvalidFormat,
    TooShort,
    TooLong,
    OutOfRange,
    TooFewItems,
    NotUnique,
}

impl From<Violation> for FieldCode {
    fn from(violation: Violation) -> Self {
        match violation {
            Violation::TooShort => FieldCode::TooShort,
            Violation::TooLong => FieldCode::TooLong,
            Violation::OutOfRange => FieldCode::OutOfRange,
            Violation::InvalidFormat => FieldCode::InvalidFormat,
            Violation::TooFewItems => FieldCode::TooFewItems,
            Violation::NotUnique => FieldCode::NotUnique,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FieldError {
    field: String,
    code: FieldCode,
    detail: String,
}

impl FieldError {
    pub fn new(field: impl Into<String>, code: FieldCode) -> Self {
        let detail = match code {
            FieldCode::Required => "is required",
            FieldCode::InvalidFormat => "has an invalid format",
            FieldCode::TooShort => "is too short",
            FieldCode::TooLong => "is too long",
            FieldCode::OutOfRange => "is out of range",
            FieldCode::TooFewItems => "has too few items",
            FieldCode::NotUnique => "contains duplicates",
        };
        FieldError { field: field.into(), code, detail: detail.to_string() }
    }
}

/// Converts one field of a request: a missing value is `required`, an invalid one
/// carries the code of its violation. The error goes into `errors`.
pub fn field<V, T>(
    value: Option<V>,
    path: &str,
    errors: &mut Vec<FieldError>,
    convert: impl FnOnce(V) -> Result<T, Violation>,
) -> Option<T> {
    let Some(value) = value else {
        errors.push(FieldError::new(path, FieldCode::Required));
        return None;
    };
    match convert(value) {
        Ok(converted) => Some(converted),
        Err(violation) => {
            errors.push(FieldError::new(path, violation.into()));
            None
        }
    }
}

// What a handler can fail with.
#[derive(Debug)]
#[allow(dead_code)] // alcune varianti servono alla seconda parte di events e a venues
pub enum AppError {
    // 422: one or more fields are missing or invalid.
    Validation(Vec<FieldError>),
    // 400: the body is not the expected JSON, or a path parameter is malformed.
    Malformed(String),
    // 404: the resource does not exist.
    NotFound(String),
    // 500: something failed on our side; the detail is logged, not sent.
    Internal(String),
    // 401: missing or invalid credentials.
    Unauthorized,
    // 403: authenticated, but not allowed.
    Forbidden(String),
    // 409
    VenueInUse(String),
    // 409
    EventPublished(String),
    // 409
    EventNotPublishable(String),
    // 409
    SalesOpen(String),
    // 422
    UnknownVenue(String),
    // 422
    UnknownSeat(String),
}

#[derive(Serialize)]
struct Problem {
    #[serde(rename = "type")]
    kind: String,
    title: &'static str,
    status: u16,
    code: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<FieldError>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        use StatusCode as S;

        let (status, code, title, detail, errors) = match self {
            AppError::Validation(errors) => {
                (S::UNPROCESSABLE_ENTITY, "validation_failed", "Validation failed", None, errors)
            }
            AppError::Malformed(detail) => {
                (S::BAD_REQUEST, "malformed_request", "Malformed request", Some(detail), Vec::new())
            }
            AppError::NotFound(detail) => (S::NOT_FOUND, "not_found", "Not found", Some(detail), Vec::new()),
            AppError::Internal(detail) => {
                tracing::error!("internal error: {detail}");
                (S::INTERNAL_SERVER_ERROR, "internal_error", "Internal error", None, Vec::new())
            }
            AppError::Unauthorized => (
                S::UNAUTHORIZED,
                "unauthorized",
                "Unauthorized",
                Some("missing or invalid credentials".to_string()),
                Vec::new(),
            ),
            AppError::Forbidden(detail) => (S::FORBIDDEN, "forbidden", "Forbidden", Some(detail), Vec::new()),
            AppError::VenueInUse(detail) => (S::CONFLICT, "venue_in_use", "Venue in use", Some(detail), Vec::new()),
            AppError::EventPublished(detail) => {
                (S::CONFLICT, "event_published", "Event published", Some(detail), Vec::new())
            }
            AppError::EventNotPublishable(detail) => {
                (S::CONFLICT, "event_not_publishable", "Event not publishable", Some(detail), Vec::new())
            }
            AppError::SalesOpen(detail) => (S::CONFLICT, "sales_open", "Sales open", Some(detail), Vec::new()),
            AppError::UnknownVenue(detail) => {
                (S::UNPROCESSABLE_ENTITY, "unknown_venue", "Unknown venue", Some(detail), Vec::new())
            }
            AppError::UnknownSeat(detail) => {
                (S::UNPROCESSABLE_ENTITY, "unknown_seat", "Unknown seat", Some(detail), Vec::new())
            }
        };

        let problem = Problem {
            kind: format!("https://ticketing.example/problems/{}", code.replace('_', "-")),
            title,
            status: status.as_u16(),
            code,
            detail,
            errors,
        };

        (status, [(header::CONTENT_TYPE, "application/problem+json")], Json(problem)).into_response()
    }
}

// Permettono `body?` negli handler che ricevono `Result<Json<T>, JsonRejection>` e simili.

impl From<JsonRejection> for AppError {
    fn from(e: JsonRejection) -> Self {
        AppError::Malformed(e.body_text())
    }
}

impl From<QueryRejection> for AppError {
    fn from(e: QueryRejection) -> Self {
        AppError::Malformed(e.body_text())
    }
}

impl From<PathRejection> for AppError {
    fn from(e: PathRejection) -> Self {
        AppError::Malformed(e.body_text())
    }
}
