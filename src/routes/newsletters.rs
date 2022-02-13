use std::fmt;

use anyhow::Context;
use axum::{extract::Extension, Json};
use http::StatusCode;
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    domain::SubscriberEmail,
    email_client::EmailClient,
    error::{Error, ResponseError},
};

#[derive(Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl fmt::Debug for PublishError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn status_code(&self) -> http::StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

// Dummy implementation
pub async fn publish_newsletter(
    Json(body): Json<BodyData>,
    Extension(pool): Extension<PgPool>,
    Extension(email_client): Extension<EmailClient>,
) -> Result<(), Error> {
    let subscribers = get_confirmed_subscribers(&pool)
        .await
        .map_err(PublishError::UnexpectedError)?;

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &body.title,
                        &body.content.html,
                        &body.content.text,
                    )
                    .await
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    })
                    .map_err(PublishError::UnexpectedError)?;
            }
            Err(error) => {
                tracing::warn!(
                error.cause_chain = ?error,
                "Skipping a confirmed subscriber. Their stored contact details are invalid"
                );
            }
        }
    }

    Ok(())
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let confirmed_subscribers =
        sqlx::query!(r#"SELECT email FROM subscriptions WHERE status = 'confirmed'"#)
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|r| {
                SubscriberEmail::try_from(r.email)
                    .map(|email| ConfirmedSubscriber { email })
                    .map_err(|e| anyhow::anyhow!(e))
            })
            .collect();
    Ok(confirmed_subscribers)
}
