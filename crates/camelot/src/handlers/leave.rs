use crate::handlers::{ReqHandler, EVENTS};
use std::future::Future;
use std::pin::Pin;
use trtcp::{Head, Request, Response};

pub(super) struct LeaveHandler;

impl ReqHandler for LeaveHandler {
    fn handle<'a>(
        &self,
        request: &'a Request<'_>,
    ) -> Pin<Box<dyn Future<Output = Response<'a>> + Send + 'a>> {
        Box::pin(async move {
            let caller_name = request.head().caller();
            let event_name = format!("{}:{}", request.action().module(), request.action().id());

            let item_position = {
                let guard = EVENTS.read().await;

                let listeners = if let Some(l) = guard.get(&event_name) {
                    l
                } else {
                    return Response::new(
                        Head::new_with_version(caller_name),
                        trtcp::Status::new(trtcp::StatusType::EventNotFound),
                        "".as_bytes(),
                    );
                };

                if let Some(p) = listeners.iter().position(|l| l == caller_name) {
                    p
                } else {
                    return Response::new(
                        Head::new_with_version(caller_name),
                        trtcp::Status::new(trtcp::StatusType::ListenerNotFound),
                        "".as_bytes(),
                    );
                }
            };

            {
                let mut guard = EVENTS.write().await;
                let listeners = if let Some(vec) = guard.get_mut(&event_name) {
                    vec
                } else {
                    return Response::new_unexpected_error(
                        caller_name,
                        "Event not found after it was found during a leave request",
                    );
                };

                listeners.swap_remove(item_position);
            }

            Response::new_ok(caller_name)
        })
    }
}
