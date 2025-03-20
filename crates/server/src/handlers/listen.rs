use crate::handlers::{basic_response, ReqHandler, LISTENERS};
use crate::head;
use std::future::Future;
use std::pin::Pin;
use trtcp::{Request, Response};

pub(super) struct ListenHandler;

impl ReqHandler for ListenHandler {
    fn handle<'a>(
        &self,
        request: Request<'a>,
    ) -> Pin<Box<dyn Future<Output = Response<'a>> + Send + 'a>> {
        Box::pin(async move {
            let caller_name = request.head().caller;
            let event_name = format!("{}:{}", request.action().module(), request.action().id());

            let mut guard = LISTENERS.write().await;

            let listeners = if let Some(l) = guard.get_mut(&event_name) {
                l
            } else {
                return Response::new(
                    head(caller_name),
                    trtcp::Status::new(trtcp::StatusType::InvalidRequest),
                    "".as_bytes(),
                );
            };

            listeners.push(caller_name.to_string());
            basic_response(caller_name)
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::handlers::LISTENERS;
    use trtcp::{Action, ActionType, Head, Request, StatusType, Version};

    #[tokio::test]
    async fn test_listen_handler() {
        let request = Request::new(
            Head::new(Version::new(1, 0), "caller"),
            Action::new(ActionType::Listen, "module", "id"),
            "".as_bytes(),
        );

        let response = ListenHandler.handle(request).await;

        assert_eq!(*response.status().r#type(), StatusType::InvalidRequest);

        let listeners = LISTENERS.read().await;

        assert!(listeners.get("module:id").is_none());
    }
}
