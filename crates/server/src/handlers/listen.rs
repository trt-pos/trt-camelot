use crate::handlers::{ReqHandler, EVENTS};
use std::future::Future;
use std::pin::Pin;
use trtcp::{Head, Request, Response};

pub(super) struct ListenHandler;

impl ReqHandler for ListenHandler {
    fn handle<'a>(
        &self,
        request: &'a Request<'_>,
    ) -> Pin<Box<dyn Future<Output = Response<'a>> + Send + 'a>> {
        Box::pin(async move {
            let caller_name = request.head().caller();
            let event_name = format!("{}:{}", request.action().module(), request.action().id());

            let already_subscribed = {
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
                
                listeners.iter().any(|l| l == caller_name)
            };
            
            if already_subscribed {
                return Response::new(
                    Head::new_with_version(caller_name),
                    trtcp::Status::new(trtcp::StatusType::AlreadySubscribed),
                    "".as_bytes(),
                );
            }

            {
                let mut guard = EVENTS.write().await;
                let listeners = if let Some(l) = guard.get_mut(&event_name) {
                    l
                } else {
                    return Response::new(
                        Head::new_with_version(caller_name),
                        trtcp::Status::new(trtcp::StatusType::EventNotFound),
                        "".as_bytes(),
                    );
                };
                listeners.push(caller_name.to_string());
                Response::new_ok(caller_name)
            }
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::handlers::EVENTS;
    use trtcp::{Action, ActionType, Head, Request, StatusType, Version};

    #[tokio::test]
    async fn test_listen_handler() {
        let request = Request::new(
            Head::new(Version::new(1, 0), "caller"),
            Action::new(ActionType::Listen, "module", "id"),
            "".as_bytes(),
        );

        let response = ListenHandler.handle(&request).await;

        assert_eq!(*response.status().r#type(), StatusType::EventNotFound);

        let listeners = EVENTS.read().await;

        assert!(listeners.get("module:id").is_none());
    }
}
