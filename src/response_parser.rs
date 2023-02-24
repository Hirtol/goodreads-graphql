use aws_config::retry::ErrorKind;
use bytes::Bytes;
use http::{Response, StatusCode};
use serde::de::DeserializeOwned;
use std::marker::PhantomData;

#[derive(thiserror::Error, Debug)]
pub enum SerdeResponseError {
    #[error("Http error on sending request to Goodreads API, response code: {0:?}")]
    HttpError(StatusCode),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
}

impl aws_config::retry::ProvideErrorKind for SerdeResponseError {
    fn retryable_error_kind(&self) -> Option<ErrorKind> {
        None
    }

    fn code(&self) -> Option<&str> {
        None
    }
}

#[derive(Debug)]
pub struct SerdeResponseParser<T> {
    _phantom: PhantomData<T>,
}

impl<T> Default for SerdeResponseParser<T> {
    fn default() -> Self {
        Self {
            _phantom: Default::default(),
        }
    }
}

impl<T> Clone for SerdeResponseParser<T> {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl<T: DeserializeOwned> aws_smithy_http::response::ParseStrictResponse
    for SerdeResponseParser<T>
{
    type Output = Result<T, SerdeResponseError>;

    fn parse(&self, response: &Response<Bytes>) -> Self::Output {
        if !response.status().is_success() {
            tracing::warn!(?response, "Goodreads query error");

            Err(SerdeResponseError::HttpError(response.status()))
        } else {
            Ok(serde_json::from_slice(response.body())?)
        }
    }
}
