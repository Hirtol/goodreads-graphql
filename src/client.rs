use crate::identity::GoodreadsCredentialsProvider;
use crate::middleware::GoodreadsMiddleware;
use crate::response_parser::SerdeResponseParser;
use aws_credential_types::cache::{CredentialsCache, SharedCredentialsCache};
use aws_credential_types::provider::SharedCredentialsProvider;
use aws_smithy_client::erase::DynConnector;
use aws_smithy_http::body::SdkBody;
use aws_types::region::Region;
use aws_types::SdkConfig;
use serde::de::DeserializeOwned;
use std::borrow::Cow;
use std::sync::Arc;

pub fn goodreads_sdk_config() -> SdkConfig {
    aws_config::SdkConfig::builder()
        .region(Region::new("us-east-1"))
        .sleep_impl(Arc::new(aws_smithy_async::rt::sleep::TokioSleep::new()))
        .credentials_cache(CredentialsCache::lazy())
        .credentials_provider(SharedCredentialsProvider::new(
            GoodreadsCredentialsProvider::new(),
        ))
        .build()
}

pub struct GoodreadsConfig {
    credentials_cache: SharedCredentialsCache,
    region: Region,
    api_endpoint: http::Uri,
}

pub struct GoodreadsClient {
    aws_client: aws_smithy_client::Client<DynConnector, GoodreadsMiddleware>,
    config: GoodreadsConfig,
}

impl GoodreadsClient {
    /// Create a new [GoodreadsClient].
    ///
    /// Use the [Default] implementation for sensible defaults instead of having to provide a custom [SdkConfig]
    pub fn new(
        custom_config: &SdkConfig,
        goodreads_api_endpoint: &str,
    ) -> Result<Self, anyhow::Error> {
        let initial_cache = custom_config
            .credentials_cache()
            .cloned()
            .unwrap_or_else(CredentialsCache::lazy);

        let shared_provider = custom_config
            .credentials_provider()
            .cloned()
            .unwrap_or_else(|| SharedCredentialsProvider::new(GoodreadsCredentialsProvider::new()));

        let config = GoodreadsConfig {
            credentials_cache: initial_cache.create_cache(shared_provider),
            region: custom_config
                .region()
                .cloned()
                .unwrap_or_else(|| Region::new("us-east-1")),
            api_endpoint: goodreads_api_endpoint.try_into()?,
        };

        let client = aws_smithy_client::Builder::new()
            .rustls_connector(Default::default())
            .middleware(GoodreadsMiddleware::default())
            .sleep_impl(
                custom_config
                    .sleep_impl()
                    .or_else(aws_smithy_async::rt::sleep::default_async_sleep)
                    .expect("No sleep runtime available"),
            )
            .build();

        Ok(Self {
            aws_client: client,
            config,
        })
    }

    #[tracing::instrument(skip(self))]
    pub async fn send_graphql_query<T>(&self, query: SdkBody) -> Result<T, anyhow::Error>
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        let request = http::Request::post(&self.config.api_endpoint).body(query)?;

        self.send_request(request).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn send_request<T: DeserializeOwned + Send + Sync + 'static>(
        &self,
        request: http::Request<SdkBody>,
    ) -> Result<T, anyhow::Error> {
        let mut final_request = aws_smithy_http::operation::Request::new(request);
        final_request
            .properties_mut()
            .insert(self.config.credentials_cache.clone());

        let op = aws_smithy_http::operation::Operation::new(
            final_request,
            SerdeResponseParser::default(),
        );

        Ok(self.aws_client.call(op).await?)
    }
}

impl Default for GoodreadsClient {
    fn default() -> Self {
        Self::new(
            &goodreads_sdk_config(),
            "https://kxbwmqov6jgg3daaamb744ycu4.appsync-api.us-east-1.amazonaws.com/graphql",
        )
        .expect("Default Goodreads config is no longer valid?")
    }
}
