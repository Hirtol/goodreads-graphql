use crate::graphql::GraphQLCustomRequest;
use crate::identity::GoodreadsCredentialsProvider;
use crate::identity_cache::GoodreadsCredentialsCache;
use crate::middleware::GoodreadsMiddleware;
use crate::response_parser::{SerdeResponseError, SerdeResponseParser};
use aws_credential_types::cache::{ProvideCachedCredentials, SharedCredentialsCache};
use aws_credential_types::provider::SharedCredentialsProvider;
use aws_credential_types::Credentials;
use aws_smithy_client::erase::DynConnector;
use aws_smithy_http::body::SdkBody;
use aws_smithy_http::result::SdkError;
use aws_types::region::Region;
use serde::de::DeserializeOwned;
use std::sync::Arc;

pub type Result<T> = std::result::Result<T, SdkError<SerdeResponseError>>;

pub struct GoodreadsClient {
    aws_client: aws_smithy_client::Client<DynConnector, GoodreadsMiddleware>,
    config: GoodreadsConfig,
}

pub struct GoodreadsConfig {
    credentials_cache: SharedCredentialsCache,
    cache: Arc<GoodreadsCredentialsCache>,
    api_endpoint: http::Uri,
}

impl GoodreadsClient {
    /// Create a new [GoodreadsClient].
    ///
    /// Use the [Default] implementation for sensible defaults.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    pub fn stored_credentials(&self) -> Option<Credentials> {
        self.config.cache.stored_credentials()
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

    /// The primary method one would use to send a GraphQL query constructed by an external query builder such as [cynic](https://github.com/obmarg/cynic).
    ///
    /// # Returns
    ///
    /// The provided `T` if the request was successful (aka, endpoint returned 200).
    /// Note that this means this `T` should expect a top level `data` and/or `error` node, as GraphQL errors are not handled
    /// in this method.
    #[tracing::instrument(skip(self, query))]
    pub async fn send_body<T>(&self, query: SdkBody) -> Result<T>
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        let request = http::Request::builder()
            .uri(&self.config.api_endpoint)
            .method(http::Method::POST)
            .header(http::header::CONTENT_TYPE, "application/x-amz-json-1.1")
            .body(query)
            .map_err(SdkError::construction_failure)?;

        self.send_request(request).await
    }

    /// The lowest level method to send a request to the Goodreads API.
    ///
    /// Note that the caller is responsible for constructing the request properly.
    /// For most use-cases [send_body](GoodreadsClient::send_body) is preferable.
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
        Self::builder()
            .build()
            .expect("Default Goodreads config is no longer valid?")
    }
}

#[derive(Default)]
pub struct ClientBuilder {
    region: Option<Region>,
    api_endpoint: Option<String>,
    saved_credentials: Option<Credentials>,
}

impl ClientBuilder {
    pub fn new() -> ClientBuilder {
        Self::default()
    }

    pub fn build(self) -> Result<GoodreadsClient> {
        let region = self.region.unwrap_or_else(|| Region::new("us-east-1"));

        let shared_provider = SharedCredentialsProvider::new(GoodreadsCredentialsProvider::new(region));

        let caches = Arc::new(GoodreadsCredentialsCache::new(shared_provider, self.saved_credentials));

        let config = GoodreadsConfig {
            credentials_cache: SharedCredentialsCache::from(caches.clone() as Arc<dyn ProvideCachedCredentials>),
            cache: caches,
            api_endpoint: self
                .api_endpoint
                .as_deref()
                .unwrap_or("https://kxbwmqov6jgg3daaamb744ycu4.appsync-api.us-east-1.amazonaws.com/graphql")
                .try_into()
                .map_err(SdkError::construction_failure)?,
        };

        let client = aws_smithy_client::Builder::new()
            .rustls_connector(Default::default())
            .middleware(GoodreadsMiddleware::default())
            .sleep_impl(aws_smithy_async::rt::sleep::default_async_sleep().expect("No sleep runtime available"))
            .build();

        Ok(GoodreadsClient {
            aws_client: client,
            config,
        })
    }

    pub fn region(mut self, region: impl Into<Option<Region>>) -> Self {
        self.set_region(region);
        self
    }

    pub fn set_region(&mut self, region: impl Into<Option<Region>>) -> &mut Self {
        self.region = region.into();
        self
    }

    pub fn api_endpoint(mut self, url: impl Into<Option<String>>) -> Self {
        self.set_api_endpoint(url);
        self
    }

    pub fn set_api_endpoint(&mut self, url: impl Into<Option<String>>) -> &mut Self {
        self.api_endpoint = url.into();
        self
    }

    pub fn credentials(mut self, creds: Option<Credentials>) -> Self {
        self.set_credentials(creds);
        self
    }

    pub fn set_credentials(&mut self, creds: Option<Credentials>) -> &mut Self {
        self.saved_credentials = creds;
        self
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
