use std::future::Future;
use std::pin::Pin;
use trtcp::{Request, Response};
use crate::handlers::ReqHandler;

pub(super) struct QueryHandler;

impl ReqHandler for QueryHandler {
    fn handle<'a>(&self, request: Request<'a>) -> Pin<Box<dyn Future<Output = Response<'a>> + Send + 'a>> {
        let body = *request.body();
        
        let response = Response::new(
            trtcp::Head::new(
                trtcp::Version::actual(),
                "server",
            ),
            trtcp::Status::new(
                trtcp::StatusType::OK,
            ),
            body,
        );
        
        Box::pin(async move {
            response
        })
    }
}