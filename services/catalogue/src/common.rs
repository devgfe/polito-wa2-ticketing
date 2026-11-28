use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, NaiveDateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, FieldCode, FieldError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Violation {
    TooShort,
    TooLong,
    OutOfRange,
    InvalidFormat,
    TooFewItems,
    NotUnique,
}

// ---------- Conversioni per `error::field` ----------
// Ognuna trasforma il valore ricevuto nel tipo giusto, o dice quale regola viola.

pub fn check_length(value: String, min: usize, max: usize) -> Result<String, Violation> {
    let n = value.chars().count();
    if n < min {
        Err(Violation::TooShort)
    } else if n > max {
        Err(Violation::TooLong)
    } else {
        Ok(value)
    }
}

// Gli id sono BIGINT positivi nel DB, stringhe nell'API.
pub fn parse_id(value: &str) -> Result<i64, Violation> {
    value
        .parse()
        .ok()
        .filter(|id| *id > 0)
        .ok_or(Violation::InvalidFormat)
}

pub fn check_currency(value: String) -> Result<String, Violation> {
    if value.len() == 3 && value.chars().all(|c| c.is_ascii_uppercase()) {
        Ok(value)
    } else {
        Err(Violation::InvalidFormat)
    }
}

// ---------- Date ----------

// `LocalDateTime` dell'OpenAPI: ora locale del venue, senza fuso.
const LOCAL_DATE_TIME: &str = "%Y-%m-%dT%H:%M:%S";

pub fn parse_local(value: &str) -> Result<NaiveDateTime, Violation> {
    NaiveDateTime::parse_from_str(value, LOCAL_DATE_TIME).map_err(|_| Violation::InvalidFormat)
}

pub fn format_local(value: NaiveDateTime) -> String {
    value.format(LOCAL_DATE_TIME).to_string()
}

// `Instant` dell'OpenAPI, es. "2026-11-04T18:00:00Z".
pub fn format_instant(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

// ---------- Paginazione ----------

#[derive(Debug, Clone, Copy)]
pub struct Pagination {
    pub default: i64,
    pub max: i64,
}

impl Pagination {
    pub fn limit(&self, requested: Option<i64>) -> Result<i64, AppError> {
        let limit = requested.unwrap_or(self.default);
        if (1..=self.max).contains(&limit) {
            Ok(limit)
        } else {
            Err(AppError::Validation(vec![FieldError::new(
                "limit",
                FieldCode::OutOfRange,
            )]))
        }
    }
}

#[derive(Serialize, Deserialize)]
struct CursorData {
    id: String,
}

pub fn encode_cursor(last_id: i64) -> String {
    let json = serde_json::to_vec(&CursorData {
        id: last_id.to_string(),
    })
    .expect("cursor serializes");
    URL_SAFE_NO_PAD.encode(json)
}

pub fn decode_cursor(cursor: &str) -> Result<i64, AppError> {
    let invalid =
        || AppError::Validation(vec![FieldError::new("cursor", FieldCode::InvalidFormat)]);

    let bytes = URL_SAFE_NO_PAD.decode(cursor).map_err(|_| invalid())?;
    let data: CursorData = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
    parse_id(&data.id).map_err(|_| invalid())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cursor_matches_the_openapi_example() {
        assert_eq!(encode_cursor(4711), "eyJpZCI6IjQ3MTEifQ");
        assert_eq!(decode_cursor("eyJpZCI6IjQ3MTEifQ").unwrap(), 4711);
    }

    #[test]
    fn a_made_up_cursor_is_rejected() {
        assert!(decode_cursor("not-a-cursor").is_err());
    }

    #[test]
    fn limit_uses_default_and_bounds() {
        let p = Pagination {
            default: 20,
            max: 200,
        };
        assert_eq!(p.limit(None).unwrap(), 20);
        assert_eq!(p.limit(Some(200)).unwrap(), 200);
        assert!(p.limit(Some(0)).is_err());
        assert!(p.limit(Some(201)).is_err());
    }

    #[test]
    fn local_date_times_round_trip() {
        let t = parse_local("2026-12-05T20:00:00").unwrap();
        assert_eq!(format_local(t), "2026-12-05T20:00:00");
        assert_eq!(
            parse_local("2026-12-05 20:00"),
            Err(Violation::InvalidFormat)
        );
    }

    #[test]
    fn ids_must_be_positive_numbers() {
        assert_eq!(parse_id("3"), Ok(3));
        assert_eq!(parse_id("0"), Err(Violation::InvalidFormat));
        assert_eq!(parse_id("abc"), Err(Violation::InvalidFormat));
    }

    #[test]
    fn length_counts_characters() {
        assert_eq!(check_length("è".into(), 1, 1), Ok("è".to_string()));
        assert_eq!(check_length("".into(), 1, 200), Err(Violation::TooShort));
        assert_eq!(check_length("abc".into(), 1, 2), Err(Violation::TooLong));
    }

    #[test]
    fn currency_must_be_three_uppercase_letters() {
        assert_eq!(check_currency("EUR".into()), Ok("EUR".to_string()));
        assert_eq!(check_currency("eur".into()), Err(Violation::InvalidFormat));
    }
}
