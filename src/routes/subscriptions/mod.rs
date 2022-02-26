pub mod confirm;

use anyhow::Context;
use axum::{
    extract::{Extension, Form},
    response::IntoResponse,
};
use chrono::Utc;
use http::StatusCode;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::Deserialize;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    domain::{EmailAddress, SubscriberName},
    email_client::EmailClient,
    startup::ApplicationBaseUrl,
};

pub async fn handler(
    Form(data): Form<FormData>,
    Extension(pool): Extension<PgPool>,
    Extension(email_client): Extension<EmailClient>,
    Extension(base_url): Extension<ApplicationBaseUrl>,
) -> Result<(), Error> {
    let mut transaction = pool
        .begin()
        .await
        .context("failed to acquire a Postgres connection from the pool")
        .map_err(Error::from)?;
    let subscriber_id = insert_subscriber(&mut transaction, &data)
        .await
        .context("failed to insert new subscriber in the database")
        .map_err(Error::from)?;
    let subscription_token = generate_subscription_token();
    store_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .context("failed to store the confirmation token for a new subscriber")
        .map_err(Error::from)?;
    transaction
        .commit()
        .await
        .context("failed to commit SQL transaction to store a new subscriber")
        .map_err(Error::from)?;
    send_confirmation_email(
        &email_client,
        &data.email,
        base_url.as_str(),
        &subscription_token,
    )
    .await
    .context("failed to send a confirmation email")
    .map_err(Error::from)?;
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct FormData {
    email: EmailAddress,
    name: SubscriberName,
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

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(transaction, new_subscriber)
)]
async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &FormData,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();

    sqlx::query!(
        r#"INSERT INTO subscriptions (id, email, name, subscribed_at, status)
            VALUES ($1, $2, $3, $4, 'pending_confirmation')"#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now(),
    )
    .execute(transaction)
    .await?;
    Ok(subscriber_id)
}

#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(transaction, subscription_token)
)]
async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO subscription_tokens(subscription_token, subscriber_id) VALUES($1, $2)",
        subscription_token,
        subscriber_id
    )
    .execute(transaction)
    .await?;
    Ok(())
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, address, base_url, token)
)]
async fn send_confirmation_email(
    email_client: &EmailClient,
    address: &EmailAddress,
    base_url: &str,
    token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!("{}/subscriptions/confirm?token={}", base_url, token);

    let html_body = format!(
        r#"Welcome to our newsletter!<br />
                Click <a href="{}">here</a> to confirm your subscription."#,
        confirmation_link
    );
    let text_body = format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link
    );

    email_client
        .send_email(address, "Welcome!", &html_body, &text_body)
        .await
}

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();

    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}
