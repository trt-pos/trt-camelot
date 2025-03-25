#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Error reading from the stream")]
    ReadingError,
    #[error("Error writing into the stream")]
    WritingError,
    #[error("Protocol error: {0}")]
    TrtcpError(#[from] trtcp::Error),
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("No data available from the stream")]
    NoData,
    #[error("Connection closed")]
    ConexionClosed,
}