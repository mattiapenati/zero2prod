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

use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};

#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let email = SubscriberEmail::try_from(value.email.as_str())?;
        let name = SubscriberName::try_from(value.name.as_str())?;

        Ok(Self { email, name })
    }
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(data, pool),
    fields(
        subscriber_email = %data.email,
        subscriber_name = %data.name,
    )
)]
pub async fn subscribe(
    Form(data): Form<FormData>,
    Extension(pool): Extension<Arc<PgPool>>,
) -> impl IntoResponse {
    let new_subscriber = match data.try_into() {
        Ok(new_subscriber) => new_subscriber,
        Err(_) => return http::StatusCode::BAD_REQUEST,
    };

    match insert_subscriber(&pool, &new_subscriber).await {
        Ok(_) => http::StatusCode::OK,
        Err(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(pool, new_subscriber)
)]
async fn insert_subscriber(
    pool: &PgPool,
    new_subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO subscriptions (id, email, name, subscribed_at) VALUES ($1, $2, $3, $4)"#,
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now(),
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(())
}
