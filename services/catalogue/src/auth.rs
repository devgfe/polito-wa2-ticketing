// PROVVISORIO. L'utente si legge dall'header `X-User-Id`.
// TODO: validare il JWT di Keycloak (header `Authorization: Bearer ...`) e usare il suo `sub`.
// Gli handler non cambieranno: cambia solo `from_request_parts` qui sotto.

use axum::{extract::FromRequestParts, http::request::Parts};
use uuid::Uuid;

use crate::error::AppError;

/// L'utente della richiesta, se c'è.
pub struct CurrentUser(pub Option<Uuid>);

impl CurrentUser {
    /// Per gli endpoint che richiedono il login: senza utente → 401.
    pub fn require(self) -> Result<Uuid, AppError> {
        self.0.ok_or(AppError::Unauthorized)
    }
}

impl<S: Send + Sync> FromRequestParts<S> for CurrentUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let Some(value) = parts.headers.get("x-user-id") else {
            return Ok(CurrentUser(None));
        };
        // Un header presente ma non valido è un errore, non un utente anonimo.
        value
            .to_str()
            .ok()
            .and_then(|v| v.parse().ok())
            .map(|id| CurrentUser(Some(id)))
            .ok_or(AppError::Unauthorized)
    }
}
