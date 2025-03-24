use crate::handlers::ReqHandler;
use std::future::Future;
use std::pin::Pin;
use trtcp::{Head, Request, Response};

pub(super) struct CallbackHandler;

impl ReqHandler for CallbackHandler {
    fn handle<'a>(
        &self,
        request: &'a Request<'_>,
    ) -> Pin<Box<dyn Future<Output = Response<'a>> + Send + 'a>> {
        Box::pin(async move {
            return Response::new(
                Head::new_with_version(request.head().caller()),
                trtcp::Status::new(trtcp::StatusType::InvalidRequest),
                "Server doesn't handle callbacks. Clients recive them when someone does an invoke request".as_bytes(),
            );
        })
    }
}
