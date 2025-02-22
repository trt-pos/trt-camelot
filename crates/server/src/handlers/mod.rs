use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, LazyLock};
use tokio::sync::RwLock;
use trtcp::{Response, Status, StatusType};

mod call;
mod create;
mod invalid;
mod leave;
mod listen;
mod query;
mod transaction;

static LISTENERS: LazyLock<Arc<RwLock<HashMap<String, Vec<String>>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

trait ReqHandler: Send {
    fn handle<'a>(
        &self,
        request: trtcp::Request<'a>,
    ) -> Pin<Box<dyn Future<Output = Response<'a>> + Send + 'a>>;
}

impl From<&trtcp::ActionType> for Box<dyn ReqHandler> {
    fn from(value: &trtcp::ActionType) -> Self {
        match value {
            trtcp::ActionType::Connect => {
                Box::from(invalid::InvalidHandler::new(StatusType::AlreadyConnected))
            }
            trtcp::ActionType::Query => Box::from(query::QueryHandler),
            trtcp::ActionType::Listen => Box::from(listen::ListenHandler),
            trtcp::ActionType::Call => Box::from(call::CallHandler),
            trtcp::ActionType::Transaction => Box::from(transaction::TransactionHandler),
            trtcp::ActionType::Leave => Box::from(leave::LeaveHandler),
            trtcp::ActionType::Create => Box::from(create::CreateHandler),
        }
    }
}

pub async fn handle_request(request: trtcp::Request<'_>) -> Response {
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

fn basic_response(caller: &str) -> Response {
    Response::new(crate::head(caller), Status::new(StatusType::OK), "")
}
