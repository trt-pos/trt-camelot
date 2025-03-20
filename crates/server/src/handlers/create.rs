use std::future::Future;
use std::pin::Pin;
use trtcp::{Request, Response};
use crate::handlers::{ReqHandler, EVENTS};
use crate::new_head;

pub(super) struct CreateHandler;

impl ReqHandler for CreateHandler {
    fn handle<'a>(&self, request: &'a Request<'_>) -> Pin<Box<dyn Future<Output = Response<'a>> + Send + 'a>> {
        Box::pin(async move { 
            let event_name = format!("{}:{}", request.action().module(), request.action().id());

            {
                let guard = EVENTS.read().await;
                
                if guard.get(&event_name).is_some() {
                    return Response::new(
                        new_head(request.head().caller()),
                        trtcp::Status::new(trtcp::StatusType::EventAlreadyExists),
                        "".as_bytes(),
                    );
                };
            }

            {
                let mut guard = EVENTS.write().await;
                guard.insert(event_name, Vec::new());
                
                super::ok_response(request.head().caller())
            }
        })
    }
}