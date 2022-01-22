use crate::{
    configuration::{DatabaseSettings, Settings},
    email_client::EmailClient,
    request_id::MakeRequestUuid,
    routes,
};

use std::{net::TcpListener, time::Duration};

use axum::{routing, AddExtensionLayer, Router};
use sqlx::postgres::{PgPool, PgPoolOptions};
use tower::ServiceBuilder;
use tower_http::{
    trace::{DefaultMakeSpan, TraceLayer},
    ServiceBuilderExt,
};
use tracing::Level;

pub struct Application {
    app: Router,
    listener: TcpListener,
}

#[derive(Clone)]
pub struct ApplicationBaseUrl(pub String);

impl ApplicationBaseUrl {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Application {
    pub fn build(settings: Settings) -> Self {
        let db_pool = get_connection_pool(&settings.database);

        let email_client = EmailClient::new(
            &settings.email_client.base_url,
            settings
                .email_client
                .sender()
                .expect("Invalid sender email address"),
            &settings.email_client.authorization_token,
            settings.email_client.timeout(),
        );

        let application_base_url = ApplicationBaseUrl(settings.application.base_url.clone());

        let middleware = ServiceBuilder::new()
            .set_x_request_id(MakeRequestUuid)
            .propagate_x_request_id()
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(
                        DefaultMakeSpan::new()
                            .include_headers(true)
                            .level(Level::INFO),
                    )
                    .on_failure(()),
            )
            .layer(AddExtensionLayer::new(db_pool))
            .layer(AddExtensionLayer::new(email_client))
            .layer(AddExtensionLayer::new(application_base_url))
            .into_inner();

        let app = Router::new()
            .route("/health_check", routing::get(routes::health_check))
            .route("/subscriptions", routing::post(routes::subscribe))
            .route("/subscriptions/confirm", routing::get(routes::confirm))
            .layer(middleware);

        let listener = TcpListener::bind(&settings.application.address()).unwrap();

        Application { app, listener }
    }

    pub async fn run(self) -> Result<(), hyper::Error> {
        hyper::Server::from_tcp(self.listener)?
            .serve(self.app.into_make_service())
            .await
    }

    pub fn address(&self) -> String {
        format!("{}", self.listener.local_addr().unwrap())
    }

    pub fn port(&self) -> u16 {
        self.listener.local_addr().unwrap().port()
    }
}

pub fn get_connection_pool(settings: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .connect_timeout(Duration::from_secs(2))
        .connect_lazy_with(settings.with_db())
}
