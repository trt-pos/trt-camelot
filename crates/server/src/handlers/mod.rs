use std::future::Future;
use std::pin::Pin;
mod call;
mod listen;
mod query;
mod transaction;

trait ReqHandler: Send {
    fn handle<'a>(
        &self,
        request: trtcp::Request<'a>,
    ) -> Pin<Box<dyn Future<Output = trtcp::Response<'a>> + Send>>;
}

impl From<&trtcp::ActionType> for Box<dyn ReqHandler> {
    fn from(value: &trtcp::ActionType) -> Self {
        match value {
            trtcp::ActionType::Query => Box::from(query::QueryHandler),
            trtcp::ActionType::Listen => Box::from(listen::ListenHandler),
            trtcp::ActionType::Call => Box::from(call::CallHandler),
            trtcp::ActionType::Transaction => Box::from(transaction::TransactionHandler)
        }
    }
}

pub async fn handle_request<'a>(request: trtcp::Request<'a>) -> trtcp::Response<'a> {
    let handler: Box<dyn ReqHandler> = request.action().r#type().into();
    let response = handler.handle(request).await;
    response
}
