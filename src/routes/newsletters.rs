use axum::Json;
use serde::Deserialize;

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

// Dummy implementation
pub async fn publish_newsletter(Json(_body): Json<BodyData>) {}
