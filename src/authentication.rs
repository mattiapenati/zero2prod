use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    body::BoxBody,
    extract::{Extension, FromRequest, RequestParts, TypedHeader},
    response::IntoResponse,
};
use futures::future::BoxFuture;
use headers::{authorization::Basic, Authorization};
use http::{header, Request, Response, StatusCode};
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
use tower_http::auth::AsyncAuthorizeRequest;
use uuid::Uuid;

use crate::telemetry::spawn_blocking_with_tracing;

#[derive(Clone)]
pub struct PasswordAuthentication {
    realm: String,
}

impl PasswordAuthentication {
    pub fn new(realm: impl Into<String>) -> Self {
        PasswordAuthentication {
            realm: realm.into(),
        }
    }
}

impl<B> AsyncAuthorizeRequest<B> for PasswordAuthentication
where
    B: Send + 'static,
{
    type RequestBody = B;
    type ResponseBody = BoxBody;
    type Future = BoxFuture<'static, Result<Request<B>, Response<BoxBody>>>;

    fn authorize(&mut self, request: Request<B>) -> Self::Future {
        let this = self.clone();

        Box::pin(async move {
            let mut request = RequestParts::new(request);

            let Extension(pool) = Extension::<PgPool>::from_request(&mut request)
                .await
                .map_err(IntoResponse::into_response)?;

            let TypedHeader(credentials) =
                TypedHeader::<Authorization<Basic>>::from_request(&mut request)
                    .await
                    .map_err(|_| unauthorized_response(&this.realm))?;

            tracing::Span::current()
                .record("username", &tracing::field::display(credentials.username()));

            let user_id = validate_credentials(credentials, &pool)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?
                .ok_or_else(|| unauthorized_response(&this.realm))?;

            tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

            request
                .try_into_request()
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())
        })
    }
}

fn unauthorized_response(realm: &str) -> Response<BoxBody> {
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(
            header::WWW_AUTHENTICATE,
            format!(r#"Basic realm="{}""#, realm),
        )
        .body(BoxBody::default())
        .unwrap()
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, pool))]
async fn validate_credentials(
    credentials: Authorization<Basic>,
    pool: &PgPool,
) -> Result<Option<Uuid>, anyhow::Error> {
    let (user_id, expected_password_hash) =
        get_stored_credentials(&credentials.username(), pool).await?;

    if spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password())
    })
    .await
    .context("failed to spawn blocking task")??
    {
        Ok(user_id)
    } else {
        Ok(None)
    }
}

#[tracing::instrument(name = "Get stored credentials", skip(username, pool))]
async fn get_stored_credentials(
    username: &str,
    pool: &PgPool,
) -> Result<(Option<uuid::Uuid>, Secret<String>), anyhow::Error> {
    let row = sqlx::query!(
        "SELECT user_id, password_hash FROM users WHERE username = $1",
        username,
    )
    .fetch_optional(pool)
    .await
    .context("failed to perform a query to retrieve stored credentials")?
    .map(|row| (row.user_id, Secret::new(row.password_hash)));

    let fake_password_hash = Secret::new("$argon2id$v=19$m=15000,t=2,p=1$gZiV/M1gPc22ElAH/Jh1Hw$CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno".to_string());

    match row {
        Some((uuid, expected_password_hash)) => Ok((Some(uuid), expected_password_hash)),
        None => Ok((None, fake_password_hash)),
    }
}

#[tracing::instrument(
    name = "Verify password hash",
    skip(expected_password_hash, password_candidate)
)]
fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password_candidate: &str,
) -> Result<bool, anyhow::Error> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format.")?;

    match Argon2::default().verify_password(password_candidate.as_bytes(), &expected_password_hash)
    {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(e) => Err(e.into()),
    }
}
