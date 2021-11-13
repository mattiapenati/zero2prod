use axum::{
    extract::{Extension, Query},
    http,
    response::IntoResponse,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct Parameters {
    token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(parameters, pool))]
pub async fn confirm(
    Query(parameters): Query<Parameters>,
    Extension(pool): Extension<PgPool>,
) -> impl IntoResponse {
    let subscriber_id = match get_subscriber_id_from_token(&pool, &parameters.token).await {
        Ok(subscriber_id) => subscriber_id,
        Err(_) => return http::StatusCode::INTERNAL_SERVER_ERROR,
    };

    match subscriber_id {
        Some(subscriber_id) => {
            if confirm_subscriber(&pool, subscriber_id).await.is_err() {
                http::StatusCode::INTERNAL_SERVER_ERROR
            } else {
                http::StatusCode::OK
            }
        }
        None => http::StatusCode::UNAUTHORIZED,
    }
}

#[tracing::instrument(name = "Get subscriber_id from token", skip(pool, token))]
async fn get_subscriber_id_from_token(
    pool: &PgPool,
    token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        "SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1",
        token
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(result.map(|r| r.subscriber_id))
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(pool, subscriber_id))]
async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE subscriptions SET status = 'confirmed' WHERE id = $1",
        subscriber_id
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(())
}
