use std::future::Future;
use std::pin::Pin;
use trtcp::{Request, Response};
use crate::handlers::ReqHandler;

pub(super) struct LeaveHandler;

impl ReqHandler for LeaveHandler {
    fn handle<'a>(&self, request: Request<'a>) -> Pin<Box<dyn Future<Output = Response<'a>> + Send + 'a>> {
        todo!()
    }
}