use std::future::Future;
use std::pin::Pin;
use trtcp::{Request, Response};
use crate::handlers::ReqHandler;

pub(super) struct ListenHandler;

impl ReqHandler for ListenHandler {
    fn handle<'a>(&self, request: Request<'a>) -> Pin<Box<dyn Future<Output = Response<'a>> + Send>> {
        todo!()
    }
}