use anyhow::Context;
use axum::{
    extract::{Extension, Query},
    response::IntoResponse,
};
use http::StatusCode;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn handler(
    Query(parameters): Query<Parameters>,
    Extension(pool): Extension<PgPool>,
) -> Result<(), Error> {
    let subscriber_id = get_subscriber_id_from_token(&pool, &parameters.token)
        .await
        .context("failed to retrieve the subscriber id associated with the provided token")
        .map_err(Error::UnexpectedError)?
        .ok_or(Error::UnknownToken)?;

    confirm_subscriber(&pool, subscriber_id)
        .await
        .context("failed to update the subscriber status to `confirmed`")
        .map_err(Error::UnexpectedError)?;

    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct Parameters {
    token: String,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    #[error("there is no subscriber associated with the provided token")]
    UnknownToken,
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        match self {
            Error::UnexpectedError(source) => {
                (StatusCode::INTERNAL_SERVER_ERROR, source.to_string()).into_response()
            }
            Error::UnknownToken => StatusCode::UNAUTHORIZED.into_response(),
        }
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
    .await?;
    Ok(result.map(|r| r.subscriber_id))
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(pool, subscriber_id))]
async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE subscriptions SET status = 'confirmed' WHERE id = $1",
        subscriber_id
    )
    .execute(pool)
    .await?;
    Ok(())
}
