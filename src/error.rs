use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum Error {
    #[error("Error sending request: {0}")]
    SendRequest(reqwest::Error),

    #[error("Error parsing request body: {0}")]
    RequestBody(anyhow::Error),

    #[error("Error reading response: {0}")]
    ResponseBody(reqwest::Error),

    #[error("Invalid header: {0}")]
    InvalidHeader(String),

    #[error("Missing form data boundary: {0}")]
    FormDataBoundaryMissing(String),

    #[error("Form part lacks a name")]
    FormPartNameMissing,

    #[error("Invalid method: {0}")]
    InvalidMethod(String),

    #[error("Invalid url '{0}': {1}")]
    InvalidUrl(String, url::ParseError),

    #[error("Empty request file")]
    EmptyRequest,

    #[error("No request line found")]
    NoRequestLine,

    #[error("Not enough parts in request line: {0}")]
    NotEnoughParts(String),
}
