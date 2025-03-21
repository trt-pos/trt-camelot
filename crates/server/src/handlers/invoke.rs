use std::future::Future;
use std::pin::Pin;
use tracing::warn;
use server::{new_head, ok_response};
use trtcp::{Call, Request, Response};
use crate::handlers::{ReqHandler, EVENTS};
use crate::CLIENTS;

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
                            new_head(request.head().caller()),
                            trtcp::Status::new(trtcp::StatusType::EventNotFound),
                            "".as_bytes(),
                        );
                    }
                };
                
                let call = Call::new(
                    new_head(caller_name),
                    request.body()
                );

                let call_bytes: Vec<u8> = call.into();
                
                let guard = CLIENTS.read().await;
                for listener in listeners.iter() {
                    let client = if let Some(c) = guard.get(listener) {
                        c
                    } else {
                        warn!("Client {} not found but is registered as a listener", listener);
                        continue
                    };
                    
                    if let Err(e) = client.write_slice(&call_bytes).await {
                        warn!("Failed to send call to client {}: {}", listener, e);
                    }
                }
                
                ok_response(caller_name)
            }
        })
    }
}