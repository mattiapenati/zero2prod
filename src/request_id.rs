use std::task::{Context, Poll};

use http::Request;
use tower::{Layer, Service};
use tower_http::trace::MakeSpan;
use uuid::Uuid;

#[derive(Clone, Copy, Debug)]
struct RequestId(Uuid);

impl std::fmt::Display for RequestId {
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl RequestId {
    fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RequestIdService<S> {
    inner: S,
}

impl<B, S> Service<Request<B>> for RequestIdService<S>
where
    S: Service<Request<B>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<B>) -> Self::Future {
        let id = RequestId::new();
        req.extensions_mut().insert(id);

        self.inner.call(req)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RequestIdLayer;

impl<S> Layer<S> for RequestIdLayer {
    type Service = RequestIdService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequestIdService { inner }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MakeSpanWithRequestId;

impl<B> MakeSpan<B> for MakeSpanWithRequestId {
    fn make_span(&mut self, request: &Request<B>) -> tracing::Span {
        match request.extensions().get::<RequestId>() {
            Some(id) => {
                tracing::debug_span!(
                    "request",
                    request_id = %id,
                    method = %request.method(),
                    uri = %request.uri(),
                    version = ?request.version(),
                )
            }
            None => {
                tracing::debug_span!(
                    "request",
                    method = %request.method(),
                    uri = %request.uri(),
                    version = ?request.version(),
                )
            }
        }
    }
}
