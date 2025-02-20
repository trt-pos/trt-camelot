use std::future::Future;
use std::pin::Pin;
use trtcp::StatusType;

mod call;
mod listen;
mod query;
mod transaction;
mod invalid;

trait ReqHandler: Send {
    fn handle<'a>(
        &self,
        request: trtcp::Request<'a>,
    ) -> Pin<Box<dyn Future<Output = trtcp::Response<'a>> + Send + 'a>>;
}

impl From<&trtcp::ActionType> for Box<dyn ReqHandler> {
    fn from(value: &trtcp::ActionType) -> Self {
        match value {
            trtcp::ActionType::Connect => Box::from(invalid::InvalidHandler::new(StatusType::AlreadyConnected)),
            trtcp::ActionType::Query => Box::from(query::QueryHandler),
            trtcp::ActionType::Listen => Box::from(listen::ListenHandler),
            trtcp::ActionType::Call => Box::from(call::CallHandler),
            trtcp::ActionType::Transaction => Box::from(transaction::TransactionHandler),
        }
    }
}

pub async fn handle_request(request: trtcp::Request<'_>) -> trtcp::Response {
    let version = &request.head().version;
    if *version.major() != 1 || *version.patch() != 0 {
        panic!(
            "Unsupported version: {}.{}",
            version.major(),
            version.patch()
        )
    }

    let handler: Box<dyn ReqHandler> = request.action().r#type().into();
    handler.handle(request).await
}
