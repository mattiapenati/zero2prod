use std::fmt;

use anyhow::Context;
use axum::extract::{Extension, Query};
use http::StatusCode;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{Error, ResponseError};

#[derive(Deserialize)]
pub struct Parameters {
    token: String,
}

#[derive(thiserror::Error)]
pub enum ConfirmationError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    #[error("there is no subscriber associated with the provided token")]
    UnknownToken,
}

impl fmt::Debug for ConfirmationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl ResponseError for ConfirmationError {
    fn status_code(&self) -> StatusCode {
        match *self {
            ConfirmationError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ConfirmationError::UnknownToken => StatusCode::UNAUTHORIZED,
        }
    }
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(parameters, pool))]
pub async fn confirm(
    Query(parameters): Query<Parameters>,
    Extension(pool): Extension<PgPool>,
) -> Result<(), Error> {
    let subscriber_id = get_subscriber_id_from_token(&pool, &parameters.token)
        .await
        .context("failed to retrieve the subscriber id associated with the provided token")
        .map_err(ConfirmationError::UnexpectedError)?
        .ok_or(ConfirmationError::UnknownToken)?;

    confirm_subscriber(&pool, subscriber_id)
        .await
        .context("failed to update the subscriber status to `confirmed`")
        .map_err(ConfirmationError::UnexpectedError)?;

    Ok(())
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
