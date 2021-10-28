use crate::routes;

use std::{net::TcpListener, sync::Arc};

use axum::{handler, AddExtensionLayer, Router, Server};
use hyper::Result;
use sqlx::PgPool;

pub async fn run(listener: TcpListener, db_poll: PgPool) -> Result<()> {
    let app = Router::new()
        .route("/health_check", handler::get(routes::health_check))
        .route("/subscriptions", handler::post(routes::subscribe))
        .layer(AddExtensionLayer::new(Arc::new(db_poll)));

    Server::from_tcp(listener)?
        .serve(app.into_make_service())
        .await
}
