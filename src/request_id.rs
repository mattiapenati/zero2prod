use http::Request;
use tower_http::request_id::{MakeRequestId, RequestId};
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub struct MakeRequestUuid;

impl MakeRequestId for MakeRequestUuid {
    fn make_request_id<B>(&mut self, _: &Request<B>) -> Option<RequestId> {
        Some(RequestId::new(Uuid::new_v4().to_string().parse().unwrap()))
    }
}
