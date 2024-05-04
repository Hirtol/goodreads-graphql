use http::HeaderValue;
use serde::de::DeserializeOwned;

use crate::credentials::cache::MemoryCache;
use crate::credentials::{CredentialsCache, CredentialsManager};
use crate::graphql::GraphQLCustomRequest;

pub const GRAPHQL_URL: &str = "https://kxbwmqov6jgg3daaamb744ycu4.appsync-api.us-east-1.amazonaws.com/graphql";

pub struct GoodreadsClient {
    client: reqwest::Client,
    config: GoodreadsConfig,
}

pub struct GoodreadsConfig {
    credentials: CredentialsManager,
    api_endpoint: reqwest::Url,
    region: String,
}

impl GoodreadsClient {
    /// Create a new [GoodreadsClient].
    ///
    /// Use the [Default] implementation for sensible defaults.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// At the moment Goodreads has their introspection turned on, convenient for us as it allows us to cheaply
    /// dump the schema.
    ///
    /// # Returns
    ///
    /// The full Goodreads schema as a [Value](serde_json::Value).
    #[tracing::instrument(skip(self))]
    pub async fn introspection(&self) -> crate::Result<serde_json::Value> {
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
    pub async fn send_graphql_query<T>(&self, query: GraphQLCustomRequest<'_>) -> crate::Result<T>
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        let body = serde_json::to_string(&query)?;

        self.send_body(body).await
    }

    /// The primary method one would use to send a GraphQL query constructed by an external query builder such as [cynic](https://github.com/obmarg/cynic).
    ///
    /// # Returns
    ///
    /// The provided `T` if the request was successful (aka, endpoint returned 200).
    /// Note that this means this `T` should expect a top level `data` and/or `error` node, as GraphQL errors are not handled
    /// in this method.
    #[tracing::instrument(skip(self, body))]
    pub async fn send_body<T, B: AsRef<[u8]> + Into<reqwest::Body>>(&self, body: B) -> crate::Result<T>
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        let credentials = self.config.credentials.credentials().await?;
        let now = chrono::Utc::now();

        let mut headers = crate::static_headers!(
            http::header::CONTENT_TYPE => "application/x-amz-json-1.1"
        );

        headers.insert(
            "x-amz-security-token",
            HeaderValue::from_str(&credentials.session_token).expect("Impossible"),
        );
        headers.insert(
            "x-amz-date",
            HeaderValue::from_str(&now.to_rfc3339()).expect("Impossible"),
        );
        headers.insert(
            http::header::HOST,
            HeaderValue::from_str(self.config.api_endpoint.host_str().expect("Need valid host in URL"))
                .expect("Impossible"),
        );

        let sign = aws_sign_v4::AwsSign::new(
            "POST",
            self.config.api_endpoint.as_str(),
            &now,
            &headers,
            &self.config.region,
            &credentials.access_key_id,
            &credentials.secret_key,
            "appsync",
            &body,
        )
        .sign();

        headers.insert(http::header::AUTHORIZATION, HeaderValue::from_str(&sign).unwrap());
        println!("HEADERS: {headers:#?}");
        // let response = self
        //     .client
        //     .post(&self.config.api_endpoint)
        //     .headers(headers)
        //     .body(body)
        //     .send()
        //     .await;
        //
        // println!("RESPONSE: {response:#?}");
        // let txt = response.unwrap().text().await;
        // println!("TEXT: {txt:?}");
        //
        // todo!()

        Ok(self
            .client
            .post(self.config.api_endpoint.clone())
            .headers(headers)
            .body(body)
            .send()
            .await?
            .json()
            .await?)
    }
}

impl Default for GoodreadsClient {
    fn default() -> Self {
        Self::builder()
            .build()
            .expect("Default Goodreads config is no longer valid?")
    }
}

pub struct ClientBuilder<T: CredentialsCache = MemoryCache> {
    client: Option<reqwest::Client>,
    region: Option<String>,
    api_endpoint: Option<String>,
    identity_pool: Option<String>,
    credentials_cache: Option<T>,
}

impl<T: CredentialsCache + Send + Sync + 'static> ClientBuilder<T> {
    pub fn new() -> ClientBuilder<T> {
        Self {
            client: None,
            region: None,
            api_endpoint: None,
            identity_pool: None,
            credentials_cache: None,
        }
    }

    pub fn build(self) -> crate::Result<GoodreadsClient> {
        let region = self.region.unwrap_or_else(|| "us-east-1".into());

        let cred_cache = self
            .credentials_cache
            .map(|cred| CredentialsManager::new(cred, self.identity_pool.clone()))
            .unwrap_or_else(|| CredentialsManager::new(MemoryCache::new(None), self.identity_pool));

        let config = GoodreadsConfig {
            credentials: cred_cache,
            api_endpoint: self
                .api_endpoint
                .and_then(|a| reqwest::Url::parse(&a).ok())
                .unwrap_or_else(|| GRAPHQL_URL.try_into().unwrap()),
            region,
        };

        Ok(GoodreadsClient {
            client: self.client.unwrap_or_default(),
            config,
        })
    }

    pub fn region(mut self, region: impl Into<Option<String>>) -> Self {
        self.set_region(region);
        self
    }

    pub fn set_region(&mut self, region: impl Into<Option<String>>) -> &mut Self {
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

    pub fn credentials_cache(mut self, cache: Option<T>) -> Self {
        self.credentials_cache = cache;

        self
    }

    pub fn set_credentials_cache(&mut self, cache: Option<T>) -> &mut Self {
        self.credentials_cache = cache;

        self
    }

    pub fn client(mut self, cache: Option<reqwest::Client>) -> Self {
        self.client = cache;

        self
    }

    pub fn set_client(&mut self, cache: Option<reqwest::Client>) -> &mut Self {
        self.client = cache;

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
