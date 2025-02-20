use std::future::Future;
use std::pin::Pin;
use trtcp::{Request, Response, StatusType};
use crate::handlers::ReqHandler;

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
    fn handle<'a>(&self, _: Request<'a>) -> Pin<Box<dyn Future<Output = Response<'a>> + Send + 'a>> {
        let status_type = self.status_type.clone();
        
        Box::pin(async move {
            Response::new(
                trtcp::Head::new(
                    trtcp::Version::actual(),
                    "server",
                ),
                trtcp::Status::new(
                    status_type,
                ),
                "",
            )
        })
    }
}