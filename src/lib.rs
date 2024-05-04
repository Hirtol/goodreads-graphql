pub mod client_2;
pub mod credentials;

// pub mod client;
pub mod graphql;
// pub mod identity;
// pub mod identity_cache;
// mod middleware;
// pub mod response_parser;
//
// pub use aws_credential_types::Credentials;
// pub use aws_smithy_http::body::SdkBody;
// pub use aws_types::region::Region;
// pub use client::*;

pub use client_2::*;

pub type Result<T> = std::result::Result<T, GoodreadsError>;

#[derive(thiserror::Error, Debug)]
pub enum GoodreadsError {
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
