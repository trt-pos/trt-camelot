#[derive(thiserror::Error, Debug)]
pub enum Error
{
    #[error("Invalid request")]
    InvalidRequest,
    #[error("Invalid head")]
    InvalidHead,
    #[error("Invalid action")]
    InvalidAction,
    #[error("Invalid body")]
    InvalidBody,
}