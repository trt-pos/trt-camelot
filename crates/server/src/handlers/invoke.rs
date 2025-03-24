use std::future::Future;
use std::pin::Pin;
use tracing::warn;
use trtcp::{Action, ActionType, Head, Request, Response};
use crate::handlers::{ReqHandler, EVENTS};
use crate::CLIENT_WRITERS;

pub(super) struct InvokeHandler;

impl ReqHandler for InvokeHandler {
    fn handle<'a>(&self, request: &'a Request<'_>) -> Pin<Box<dyn Future<Output = Response<'a>> + Send + 'a>> {
        Box::pin(async move {
            let event_name = format!("{}:{}", request.action().module(), request.action().id());
            let caller_name = request.head().caller();
            
            {
                let events_guard = EVENTS.read().await;

                let listeners = {
                    if let Some(l) = events_guard.get(&event_name) {
                        l
                    } else {
                        return Response::new(
                            Head::new_with_version(request.head().caller()),
                            trtcp::Status::new(trtcp::StatusType::EventNotFound),
                            "".as_bytes(),
                        );
                    }
                };
                
                let callback_request = Request::new(
                    Head::new_with_version(caller_name),
                    Action::new(ActionType::Callback, request.action().module(), request.action().id()),
                    *request.body()
                );

                let call_bytes: Vec<u8> = callback_request.into();
                
                let guard = CLIENT_WRITERS.read().await;
                for listener in listeners.iter() {
                    let mut writer = if let Some(c) = guard.get(listener) {
                        c.lock().await
                    } else {
                        warn!("Client {} not found but is registered as a listener", listener);
                        continue
                    };
                    
                    if let Err(e) = writer.write_slice(&call_bytes).await {
                        warn!("Failed to send callback_request to client {}: {}", listener, e);
                    }
                }
                
                Response::new_ok(caller_name)
            }
        })
    }
}