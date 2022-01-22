use axum::{
    response::{IntoResponse, Response},
    Json,
};
use http::StatusCode;
use serde::Serialize;

pub trait ResponseError: std::error::Error {
    fn status_code(&self) -> StatusCode;
}

#[derive(Debug)]
pub struct Error {
    reason: Box<dyn ResponseError>,
}

impl<T: ResponseError + 'static> From<T> for Error {
    fn from(e: T) -> Self {
        Error {
            reason: Box::new(e),
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status_code = self.reason.status_code();
        let errors = {
            let mut errors = vec![self.reason.to_string()];

            let mut current = self.reason.source();
            while let Some(reason) = current {
                errors.push(format!("{}", reason));
                current = reason.source();
            }

            errors
        };

        tracing::error!(errors = ?errors, status_code = %status_code, "response failed");

        #[derive(Serialize)]
        struct Response {
            code: u16,
            message: String,
            errors: Vec<String>,
        }

        (
            status_code,
            Json(Response {
                code: status_code.as_u16(),
                message: status_code.as_str().to_string(),
                errors,
            }),
        )
            .into_response()
    }
}
