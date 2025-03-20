#[derive(thiserror::Error, Debug)]
pub enum Error
{
    #[error("Invalid request")]
    InvalidRequest,
    #[error("Invalid head section")]
    InvalidHead,
    #[error("Invalid action section")]
    InvalidAction,
    #[error("Invalid body section")]
    InvalidBody,
    #[error("Invalid response")]
    InvalidResponse,
    #[error("Invalid status section")]
    InvalidStatus,
    #[error("Invalid action type")]
    InvalidActionType,
    #[error("Invalid call")]
    InvalidCall,
}