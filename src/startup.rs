use crate::{
    request_id::{MakeSpanWithRequestId, RequestIdLayer},
    routes,
};

use std::{net::TcpListener, sync::Arc};

use axum::{handler, AddExtensionLayer, Router};
use hyper::{Result, Server};
use sqlx::PgPool;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

pub async fn run(listener: TcpListener, db_pool: PgPool) -> Result<()> {
    let middleware = ServiceBuilder::new()
        .layer(AddExtensionLayer::new(Arc::new(db_pool)))
        .layer(RequestIdLayer)
        .layer(TraceLayer::new_for_http().make_span_with(MakeSpanWithRequestId))
        .into_inner();

    let app = Router::new()
        .route("/health_check", handler::get(routes::health_check))
        .route("/subscriptions", handler::post(routes::subscribe))
        .layer(middleware);

    Server::from_tcp(listener)?
        .serve(app.into_make_service())
        .await
}
