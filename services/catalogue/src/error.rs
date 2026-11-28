use axum::{
    Json,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::common::Violation;

/// The code of an invalid field, as the OpenAPI lists it in `FieldError.code`.
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

/// One invalid field of a request: an item of `errors` in a validation problem.
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
/// carries the code of its violation. The error goes into `errors`; it never stops early.
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

/// What a handler can fail with. Each case becomes an RFC 9457 problem.
#[derive(Debug)]
pub enum AppError {
    /// 422: one or more fields are missing or invalid.
    Validation(Vec<FieldError>),
    /// 400: the body is not the expected JSON, or a path parameter is malformed.
    Malformed(String),
    /// 404: the resource does not exist.
    NotFound(String),
    /// 500: something failed on our side; the detail is logged, not sent.
    Internal(String),
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
        let (status, code, title, detail, errors) = match self {
            AppError::Validation(errors) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_failed",
                "Validation failed",
                None,
                errors,
            ),
            AppError::Malformed(detail) => (
                StatusCode::BAD_REQUEST,
                "malformed_request",
                "Malformed request",
                Some(detail),
                Vec::new(),
            ),
            AppError::NotFound(detail) => (
                StatusCode::NOT_FOUND,
                "not_found",
                "Not found",
                Some(detail),
                Vec::new(),
            ),
            AppError::Internal(detail) => {
                tracing::error!("internal error: {detail}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "Internal error",
                    None,
                    Vec::new(),
                )
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