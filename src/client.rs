use crate::graphql::GraphQLCustomRequest;
use crate::identity::GoodreadsCredentialsProvider;
use crate::middleware::GoodreadsMiddleware;
use crate::response_parser::{SerdeResponseError, SerdeResponseParser};
use aws_credential_types::cache::{CredentialsCache, SharedCredentialsCache};
use aws_credential_types::provider::SharedCredentialsProvider;
use aws_smithy_client::erase::DynConnector;
use aws_smithy_http::body::SdkBody;
use aws_smithy_http::result::SdkError;
use aws_types::region::Region;
use aws_types::SdkConfig;
use serde::de::DeserializeOwned;
use std::sync::Arc;

pub type Result<T> = std::result::Result<T, SdkError<SerdeResponseError>>;

pub fn goodreads_sdk_config() -> SdkConfig {
    let default_region = Region::new("us-east-1");

    SdkConfig::builder()
        .region(default_region.clone())
        .sleep_impl(Arc::new(aws_smithy_async::rt::sleep::TokioSleep::new()))
        .credentials_cache(CredentialsCache::lazy())
        .credentials_provider(SharedCredentialsProvider::new(GoodreadsCredentialsProvider::new(
            default_region,
        )))
        .build()
}

pub struct GoodreadsClient {
    aws_client: aws_smithy_client::Client<DynConnector, GoodreadsMiddleware>,
    config: GoodreadsConfig,
}

pub struct GoodreadsConfig {
    credentials_cache: SharedCredentialsCache,
    api_endpoint: http::Uri,
}

impl GoodreadsClient {
    /// Create a new [GoodreadsClient].
    ///
    /// Use the [Default] implementation for sensible defaults instead of having to provide a custom [SdkConfig]
    pub fn new(custom_config: &SdkConfig, goodreads_api_endpoint: &str) -> Result<Self> {
        let region = custom_config
            .region()
            .cloned()
            .unwrap_or_else(|| Region::new("us-east-1"));

        let initial_cache = custom_config
            .credentials_cache()
            .cloned()
            .unwrap_or_else(CredentialsCache::lazy);

        let shared_provider = custom_config
            .credentials_provider()
            .cloned()
            .unwrap_or_else(|| SharedCredentialsProvider::new(GoodreadsCredentialsProvider::new(region.clone())));

        let config = GoodreadsConfig {
            credentials_cache: initial_cache.create_cache(shared_provider),
            api_endpoint: goodreads_api_endpoint
                .try_into()
                .map_err(SdkError::construction_failure)?,
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

    /// At the moment Goodreads has their introspection turned on, convenient for us as it allows us to cheaply
    /// dump the schema.
    ///
    /// # Returns
    ///
    /// The full Goodreads schema as a [Value](serde_json::Value).
    #[tracing::instrument(skip(self))]
    pub async fn introspection(&self) -> Result<serde_json::Value> {
        let request = GraphQLCustomRequest::from_query(INTROSPECTION_QUERY, "IntrospectionQuery");

        self.send_graphql_query(request).await
    }

    /// Send a manually constructed [GraphQLCustomRequest]. This is mainly here for simple testing purposes.
    ///
    /// For a real application integrating this client alongside something like [cynic](https://cynic-rs.dev/overview.html) is recommended.
    ///
    /// # Returns
    ///
    /// The provided `T` if the request was successful (aka, endpoint returned 200).
    /// Note that this means this `T` should expect a top level `data` and/or `error` node, as GraphQL errors are not handled
    /// in this method.
    #[tracing::instrument(skip(self))]
    pub async fn send_graphql_query<T>(&self, query: GraphQLCustomRequest<'_>) -> Result<T>
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        let body = SdkBody::from(serde_json::to_string(&query).map_err(SdkError::construction_failure)?);

        self.send_body(body).await
    }

    #[tracing::instrument(skip(self, query))]
    pub async fn send_body<T>(&self, query: SdkBody) -> Result<T>
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        let request = http::Request::post(&self.config.api_endpoint)
            .body(query)
            .map_err(SdkError::construction_failure)?;

        self.send_request(request).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn send_request<T>(&self, request: http::Request<SdkBody>) -> Result<T>
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        let mut final_request = aws_smithy_http::operation::Request::new(request);
        final_request
            .properties_mut()
            .insert(self.config.credentials_cache.clone());

        let op = aws_smithy_http::operation::Operation::new(final_request, SerdeResponseParser::default());

        self.aws_client.call(op).await
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

static INTROSPECTION_QUERY: &str = r#"
    query IntrospectionQuery {
      __schema {
        queryType { name }
        mutationType { name }
        subscriptionType { name }
        types {
          ...FullType
        }
        directives {
          name
          description
          
          locations
          args {
            ...InputValue
          }
        }
      }
    }

    fragment FullType on __Type {
      kind
      name
      description
      fields(includeDeprecated: true) {
        name
        description
        args {
          ...InputValue
        }
        type {
          ...TypeRef
        }
        isDeprecated
        deprecationReason
      }
      inputFields {
        ...InputValue
      }
      interfaces {
        ...TypeRef
      }
      enumValues(includeDeprecated: true) {
        name
        description
        isDeprecated
        deprecationReason
      }
      possibleTypes {
        ...TypeRef
      }
    }

    fragment InputValue on __InputValue {
      name
      description
      type { ...TypeRef }
      defaultValue
    }

    fragment TypeRef on __Type {
      kind
      name
      ofType {
        kind
        name
        ofType {
          kind
          name
          ofType {
            kind
            name
            ofType {
              kind
              name
              ofType {
                kind
                name
                ofType {
                  kind
                  name
                  ofType {
                    kind
                    name
                  }
                }
              }
            }
          }
        }
      }
    }
  "#;
