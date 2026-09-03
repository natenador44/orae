//! Generic error type for axum handlers.
//!
//! - Any error convertible into `anyhow::Error` converts into `HandlerError`
//!   via `?` (blanket `From` impl below).
//! - Each error gets a UUID and a captured `SpanTrace` (the chain of
//!   `#[tracing::instrument]`-annotated spans active when the error occurred).
//! - Pair fallible handlers with `#[tracing::instrument(err(Debug))]` and
//!   tracing will automatically debug-log the error (id, description, span
//!   trace) with no manual logging calls needed.
//! - `IntoResponse` only ever exposes status + message + id to the client;
//!   internals stay in the logs.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use std::fmt;
use tracing_error::SpanTrace;
use uuid::Uuid;

pub type HandlerResult<T> = Result<T, HandlerError>;

pub struct HandlerError {
    id: Uuid,
    status: StatusCode,
    /// Client-facing message. Keep this free of internal detail.
    message: String,
    /// Full error, for logging only.
    source: anyhow::Error,
    /// Chain of active tracing spans at the point the error was created.
    span_trace: SpanTrace,
}

impl HandlerError {
    /// Build an error with an explicit status + client-facing message.
    /// Use for "expected" errors (validation, not found, auth, etc).
    pub fn with_status(status: StatusCode, message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            id: Uuid::new_v4(),
            status,
            source: anyhow::anyhow!(message.clone()),
            message,
            span_trace: SpanTrace::capture(),
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
}

/// Drives what `#[instrument(err(Debug))]` logs: id, description, causal
/// chain, and the span trace showing where the error originated.
impl fmt::Debug for HandlerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "HandlerError {{ id: {}, status: {} }}",
            self.id, self.status
        )?;
        writeln!(f, "description: {}", self.source)?;
        for cause in self.source.chain().skip(1) {
            writeln!(f, "caused by: {cause}")?;
        }
        write!(f, "span trace:\n{}", self.span_trace)
    }
}

/// Blanket conversion: anything that can become an `anyhow::Error`
/// (which is essentially any `std::error::Error + Send + Sync + 'static`,
/// plus `anyhow::Error` itself) converts into `HandlerError` via `?`.
/// Defaults to 500; use `.status(...)` (below) or `with_status` to override.
impl<E> From<E> for HandlerError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self {
            id: Uuid::new_v4(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "internal server error".to_string(),
            source: err.into(),
            span_trace: SpanTrace::capture(),
        }
    }
}

/// Lets you attach a status code to a `Result` before the `?`, e.g.
/// `db_lookup(id).await.status(StatusCode::NOT_FOUND)?`
pub trait ResultExt<T> {
    fn status(self, status: StatusCode) -> Result<T, HandlerError>;
}

impl<T, E> ResultExt<T> for Result<T, E>
where
    E: Into<anyhow::Error>,
{
    fn status(self, status: StatusCode) -> Result<T, HandlerError> {
        self.map_err(|e| {
            let mut err = HandlerError::from(e);
            err.status = status;
            err
        })
    }
}

/// Lets you turn an `Option<T>` directly into a `Result<T, HandlerError>`,
/// e.g. `db.find(id).await.or_404()?` or, with a custom message,
/// `db.find(id).await.or_404_msg("widget not found")?`.
pub trait OptionExt<T> {
    fn or_404(self) -> Result<T, HandlerError>;
    fn or_404_msg(self, message: impl Into<String>) -> Result<T, HandlerError>;
    fn ok_or_status(
        self,
        status: StatusCode,
        message: impl Into<String>,
    ) -> Result<T, HandlerError>;
}

impl<T> OptionExt<T> for Option<T> {
    fn or_404(self) -> Result<T, HandlerError> {
        self.ok_or_status(StatusCode::NOT_FOUND, "not found")
    }

    fn or_404_msg(self, message: impl Into<String>) -> Result<T, HandlerError> {
        self.ok_or_status(StatusCode::NOT_FOUND, message)
    }

    fn ok_or_status(
        self,
        status: StatusCode,
        message: impl Into<String>,
    ) -> Result<T, HandlerError> {
        self.ok_or_else(|| HandlerError::with_status(status, message))
    }
}

#[derive(Serialize)]
struct ErrorBody {
    error_id: Uuid,
    message: String,
}

impl IntoResponse for HandlerError {
    fn into_response(self) -> Response {
        let body = ErrorBody {
            error_id: self.id,
            message: self.message,
        };
        (self.status, Json(body)).into_response()
    }
}

// ---------------------------------------------------------------------
// Example usage
// ---------------------------------------------------------------------
//
// // subscriber setup (once, at startup):
// use tracing_error::ErrorLayer;
// use tracing_subscriber::{prelude::*, EnvFilter};
//
// tracing_subscriber::registry()
//     .with(EnvFilter::from_default_env())
//     .with(tracing_subscriber::fmt::layer())
//     .with(ErrorLayer::default()) // <- required for SpanTrace::capture() to work
//     .init();
//
// // a handler:
// #[tracing::instrument(err(Debug))]
// async fn get_widget(
//     Path(id): Path<Uuid>,
// ) -> Result<Json<Widget>, HandlerError> {
//     let widget = db_lookup(id)
//         .await
//         .status(StatusCode::NOT_FOUND)?; // expected error, explicit status
//
//     let extra = some_other_fallible_call()?; // unexpected error, defaults to 500
//
//     Ok(Json(widget))
// }
