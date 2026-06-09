use core::fmt;

#[derive(Debug)]
pub enum TleError {
    Network(ureq::Error),
    ReadFailure(std::io::Error),
    DeserializationFail(serde_json::Error),
    DateTimeParseError(chrono::ParseError)
}

impl fmt::Display for TleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TleError::Network(err) => write!(f, "Network request failed: {err}"),
            TleError::ReadFailure(err) => write!(f, "Failed to read data payload: {err}"),
            TleError::DeserializationFail(err) => write!(f, "Failed to deserialize payload: {err}"),
            TleError::DateTimeParseError(err) => write!(f, "Error while trying to parse date: {err}"),
        }
    }
}

impl std::error::Error for TleError {}

impl From<ureq::Error> for TleError {
    fn from(err: ureq::Error) -> Self {
        TleError::Network(err)
    }
}

impl From<std::io::Error> for TleError {
    fn from(err: std::io::Error) -> Self {
        TleError::ReadFailure(err)
    }
}

impl From<serde_json::Error> for TleError {
    fn from(err: serde_json::Error) -> Self {
        TleError::DeserializationFail(err)
    }
}

impl From<chrono::ParseError> for TleError {
    fn from(err: chrono::ParseError) -> Self {
        TleError::DateTimeParseError(err)
    }
}