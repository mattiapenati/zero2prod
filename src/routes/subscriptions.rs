use axum::{
    extract::{Extension, Form},
    http,
    response::IntoResponse,
};
use chrono::Utc;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::Deserialize;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    startup::ApplicationBaseUrl,
};

#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let email = SubscriberEmail::try_from(value.email)?;
        let name = SubscriberName::try_from(value.name)?;

        Ok(Self { email, name })
    }
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(data, pool, email_client, base_url),
    fields(
        subscriber_email = %data.email,
        subscriber_name = %data.name,
    )
)]
pub async fn subscribe(
    Form(data): Form<FormData>,
    Extension(pool): Extension<PgPool>,
    Extension(email_client): Extension<EmailClient>,
    Extension(base_url): Extension<ApplicationBaseUrl>,
) -> impl IntoResponse {
    let new_subscriber = match data.try_into() {
        Ok(new_subscriber) => new_subscriber,
        Err(e) => {
            tracing::error!("Invalid request body: {:?}", e);
            return http::StatusCode::BAD_REQUEST;
        }
    };

    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(e) => {
            tracing::error!("Failed to begin transaction: {:?}", e);
            return http::StatusCode::INTERNAL_SERVER_ERROR;
        }
    };

    let subscriber_id = match insert_subscriber(&mut transaction, &new_subscriber).await {
        Ok(subscriber_id) => subscriber_id,
        Err(_) => return http::StatusCode::INTERNAL_SERVER_ERROR,
    };

    let subscription_token = generate_subscription_token();
    if store_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .is_err()
    {
        return http::StatusCode::INTERNAL_SERVER_ERROR;
    }

    if let Err(e) = transaction.commit().await {
        tracing::error!("Failed to commit transaction: {:?}", e);
        return http::StatusCode::INTERNAL_SERVER_ERROR;
    }

    if send_confirmation_email(
        &email_client,
        new_subscriber,
        base_url.as_str(),
        &subscription_token,
    )
    .await
    .is_err()
    {
        return http::StatusCode::INTERNAL_SERVER_ERROR;
    }

    http::StatusCode::OK
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(transaction, new_subscriber)
)]
async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
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
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

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
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(())
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, new_subscriber, base_url, token)
)]
async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
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
        .send_email(new_subscriber.email, "Welcome!", &html_body, &text_body)
        .await
        .map_err(|e| {
            tracing::error!("Failed to send confirmation email: {:?}", e);
            e
        })
}

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();

    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}
