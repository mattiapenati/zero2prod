use anyhow::Context;
use axum::{extract::Extension, response::IntoResponse, Json};
use http::StatusCode;
use serde::Deserialize;
use sqlx::PgPool;

use crate::{domain::EmailAddress, email_client::EmailClient};

// Dummy implementation
pub async fn handler(
    Json(body): Json<BodyData>,
    Extension(pool): Extension<PgPool>,
    Extension(email_client): Extension<EmailClient>,
) -> Result<(), Error> {
    let subscribers = get_confirmed_subscribers(&pool)
        .await
        .map_err(Error::UnexpectedError)?;

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
                    .map_err(Error::UnexpectedError)?;
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

#[derive(Debug, Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(Debug, Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        match self {
            Error::UnexpectedError(source) => {
                (StatusCode::INTERNAL_SERVER_ERROR, source.to_string()).into_response()
            }
        }
    }
}

struct ConfirmedSubscriber {
    email: EmailAddress,
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
                r.email
                    .parse::<EmailAddress>()
                    .map(|email| ConfirmedSubscriber { email })
                    .map_err(|e| anyhow::anyhow!(e))
            })
            .collect();
    Ok(confirmed_subscribers)
}
