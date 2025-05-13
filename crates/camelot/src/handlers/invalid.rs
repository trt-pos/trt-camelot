use crate::handlers::ReqHandler;
use std::future::Future;
use std::pin::Pin;
use trtcp::{Request, Response, StatusType};

pub(super) struct InvalidHandler {
    status_type: StatusType,
}

impl InvalidHandler {
    pub fn new(status_type: StatusType) -> Self {
        Self {
            status_type,
        }
    }
}

impl ReqHandler for InvalidHandler {
    fn handle<'a>(&self, request: &'a Request<'_>) -> Pin<Box<dyn Future<Output = Response<'a>> + Send + 'a>> {
        let status_type = self.status_type.clone();
        
        Box::pin(async move {
            Response::new(
                trtcp::Head::new(
                    trtcp::Version::actual(),
                    request.head().caller(),
                ),
                trtcp::Status::new(
                    status_type,
                ),
                "".as_bytes(),
            )
        })
    }
}