use std::sync::Arc;

use axum::{
    extract::{Extension, Form},
    http,
    response::IntoResponse,
};
use chrono::Utc;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

pub async fn subscribe(
    Form(form): Form<FormData>,
    Extension(pool): Extension<Arc<PgPool>>,
) -> impl IntoResponse {
    match sqlx::query!(
        r#"INSERT INTO subscriptions (id, email, name, subscribed_at) VALUES ($1, $2, $3, $4)"#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now(),
    )
    .execute(pool.as_ref())
    .await
    {
        Ok(_) => http::StatusCode::OK,
        Err(e) => {
            println!("Failed to execute query: {}", e);
            http::StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
