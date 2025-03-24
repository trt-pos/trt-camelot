use std::future::Future;
use std::pin::Pin;
use trtcp::{Head, Request, Response};
use crate::handlers::{ReqHandler, EVENTS};

pub(super) struct CreateHandler;

impl ReqHandler for CreateHandler {
    fn handle<'a>(&self, request: &'a Request<'_>) -> Pin<Box<dyn Future<Output = Response<'a>> + Send + 'a>> {
        Box::pin(async move { 
            let event_name = format!("{}:{}", request.action().module(), request.action().id());

            {
                let guard = EVENTS.read().await;
                
                if guard.get(&event_name).is_some() {
                    return Response::new(
                        Head::new_with_version(request.head().caller()),
                        trtcp::Status::new(trtcp::StatusType::EventAlreadyExists),
                        "".as_bytes(),
                    );
                };
            }

            {
                let mut guard = EVENTS.write().await;
                guard.insert(event_name, Vec::new());
                
                Response::new_ok(request.head().caller())
            }
        })
    }
}