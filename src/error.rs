use thiserror::Error;

#[allow(missing_docs)]
#[derive(Error, Debug)]
pub enum Error {
    #[allow(dead_code)]
    #[error("Unknown Error")]
    Unknown,

    #[error(transparent)]
    CApiError(#[from] std::ffi::NulError),

    #[error("While {action}: {message}")]
    Exception { action: String, message: String },
}

unsafe impl Send for Error {}
