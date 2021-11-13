use axum::{http, response::IntoResponse};

#[tracing::instrument(name = "Health check", level = "trace")]
pub async fn health_check() -> impl IntoResponse {
    http::StatusCode::OK
}
