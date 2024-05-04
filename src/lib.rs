pub mod client;
pub mod credentials;
pub mod graphql;

pub use client::*;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to refresh credentials")]
    FailedCredentials,
    #[error("Failed to build request")]
    RequestError,
    #[error(transparent)]
    HttpError(#[from] reqwest::Error),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

macro_rules! static_headers {
    ($($name:expr => $val:expr $(,)?)*) => {
        {
            let mut headers = reqwest::header::HeaderMap::new();

            $(
            headers.insert($name, http::header::HeaderValue::from_static($val));
            )*

            headers
        }
    };
}

use static_headers;
