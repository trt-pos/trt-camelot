use crate::handlers::ReqHandler;
use server::new_head;
use std::future::Future;
use std::pin::Pin;
use trtcp::{Request, Response};

pub(super) struct CallbackHandler;

impl ReqHandler for CallbackHandler {
    fn handle<'a>(
        &self,
        request: &'a Request<'_>,
    ) -> Pin<Box<dyn Future<Output = Response<'a>> + Send + 'a>> {
        Box::pin(async move {
            return Response::new(
                new_head(request.head().caller()),
                trtcp::Status::new(trtcp::StatusType::InvalidRequest),
                "Server doesn't handle callbacks. Clients recive them when someone does an invoke request".as_bytes(),
            );
        })
    }
}
