use anyhow::Context;
use axum::{
    body::BoxBody,
    extract::{Extension, FromRequest, RequestParts, TypedHeader},
    response::IntoResponse,
};
use futures::future::BoxFuture;
use headers::{authorization::Basic, Authorization};
use http::{header, Request, Response, StatusCode};
use sqlx::PgPool;
use tower_http::auth::AsyncAuthorizeRequest;
use uuid::Uuid;

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

            let user_id = validate_credentials(&credentials, &pool)
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

async fn validate_credentials(
    credentials: &Authorization<Basic>,
    pool: &PgPool,
) -> Result<Option<Uuid>, anyhow::Error> {
    let user_id: Option<_> = sqlx::query!(
        "SELECT user_id FROM users WHERE username = $1 AND password = $2",
        credentials.username(),
        credentials.password()
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to validate auth credentials.")?
    .map(|r| r.user_id);

    Ok(user_id)
}
