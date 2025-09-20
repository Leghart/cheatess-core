use thiserror::Error;

pub type CheatessResult<T> = Result<T, CheatessError>;

#[derive(Error, Debug)]
pub enum CheatessError {
    #[error("Monitor not found")]
    MonitorNotFound,

    #[error("Xcap error: {0}")]
    XcapError(#[from] xcap::XCapError),

    #[error("OpenCV error: {0}")]
    OpenCVError(#[from] opencv::Error),

    #[error("Detected too many position changes")]
    TooManyPositionChanges,

    #[error("Detected no move")]
    NoMoveDetected,

    #[error("Invalid amount of moves detected: {0}")]
    InvalidAmountOfMoves(usize),

    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),
}
