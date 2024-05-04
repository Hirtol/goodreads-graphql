use std::sync::Arc;
use std::time::SystemTime;

use futures::future::BoxFuture;
use tokio::sync::RwLock;

pub use cache::*;

pub const GOODREADS_IDENTITY_POOL: &str = "us-east-1:16da77fa-4392-4d35-bd47-bb0e2d3f73be";
const COGNITO_IDENT_URL: &str = "https://cognito-identity.us-east-1.amazonaws.com/";

#[serde_with::serde_as]
#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Credentials {
    pub access_key_id: String,
    pub session_token: String,
    pub secret_key: String,
    /// Credential Expiry
    ///
    /// A SystemTime at which the credentials should no longer be used because they have expired.
    /// The primary purpose of this value is to allow credentials to communicate to the caching
    /// provider when they need to be refreshed.
    ///
    /// If these credentials never expire, this value will be set to `None`
    #[serde_as(as = "Option<serde_with::TimestampSecondsWithFrac<f64>>")]
    pub expiration: Option<SystemTime>,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CredentialsResponse {
    credentials: Credentials,
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
#[serde_with::serde_as]
pub struct Identity {
    pub identity_id: String,
}

pub struct CredentialsManager {
    client: reqwest::Client,
    cache: Box<RwLock<dyn CredentialsCache + Send + Sync>>,
    identity_pool: String,
    refresh_lock: std::sync::Mutex<()>,
}

impl CredentialsManager {
    pub fn new(
        credentials_cache: impl CredentialsCache + Send + Sync + 'static,
        identity_pool_id: Option<String>,
    ) -> Self {
        Self {
            identity_pool: identity_pool_id.unwrap_or_else(|| GOODREADS_IDENTITY_POOL.into()),
            client: reqwest::Client::new(),
            cache: Box::new(RwLock::new(credentials_cache)),
            refresh_lock: std::sync::Mutex::new(()),
        }
    }

    pub async fn credentials(&self) -> crate::Result<Arc<Credentials>> {
        let credentials = self.cache.read().await.request_credentials().await?;

        match credentials {
            Some(creds)
                if creds
                    .expiration
                    .map(|expire| SystemTime::now() < expire)
                    .unwrap_or(false) =>
            {
                Ok(creds)
            }
            _ => {
                // Need to refresh the credentials.
                // Probably better done with a separate task, but this ensures only a single task actually does the refresh.
                match self.refresh_lock.try_lock() {
                    Ok(_lock) => self.force_refresh().await,
                    Err(_) => {
                        // Wait for the refresh to occur above on a different task
                        drop(self.refresh_lock.lock());
                        // If the above failed, then we'll just error out here and let the caller take care of it.
                        self.cache
                            .read()
                            .await
                            .request_credentials()
                            .await?
                            .ok_or(crate::Error::FailedCredentials)
                    }
                }
            }
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn force_refresh(&self) -> crate::Result<Arc<Credentials>> {
        let new_credentials = Arc::new(self.fetch_credentials().await?);
        self.cache
            .write()
            .await
            .persist_credentials(new_credentials.clone())
            .await?;

        Ok(new_credentials)
    }

    #[tracing::instrument(skip(self))]
    async fn fetch_credentials(&self) -> crate::Result<Credentials> {
        let identity_id = self.fetch_identity_id().await?;
        let headers = crate::static_headers!(
            "x-amz-target" => "AWSCognitoIdentityService.GetCredentialsForIdentity",
            "x-amz-user-agent" => "aws-sdk-js/2.711.0 callback",
            http::header::CONTENT_TYPE => "application/x-amz-json-1.1"
        );

        let new_credentials = self
            .client
            .post(COGNITO_IDENT_URL)
            .headers(headers)
            .json(&identity_id)
            .send()
            .await?
            .json()
            .await?;

        Ok(new_credentials)
    }

    async fn fetch_identity_id(&self) -> crate::Result<Identity> {
        let headers = crate::static_headers!(
            "x-amz-target" => "AWSCognitoIdentityService.GetId",
            "x-amz-user-agent" => "aws-sdk-js/2.711.0 callback",
            http::header::CONTENT_TYPE => "application/x-amz-json-1.1"
        );

        let out = self
            .client
            .post(COGNITO_IDENT_URL)
            .headers(headers)
            .json(&serde_json::json!({
                "IdentityPoolId": self.identity_pool
            }))
            .send()
            .await?
            .json()
            .await?;

        Ok(out)
    }
}

pub trait CredentialsCache {
    /// Request the currently cached credentials, note that this will be called on each request,
    /// so it should be cheap to execute.
    ///
    /// The client will check for expiration.
    ///
    /// # Returns
    ///
    /// `Some(credentials)` if the credentials exist, `None` if they don't exist and should be re-acquired, or `Err` if something went wrong.
    fn request_credentials(&self) -> BoxFuture<crate::Result<Option<Arc<Credentials>>>>;

    /// Persist the newly acquired credentials in the cache.
    fn persist_credentials(&mut self, credentials: Arc<Credentials>) -> BoxFuture<crate::Result<()>>;
}

pub mod cache {
    use std::io::BufWriter;
    use std::path::PathBuf;
    use std::sync::Arc;

    use futures::future::BoxFuture;
    use futures::FutureExt;

    use crate::credentials::Credentials;

    pub struct MemoryCache {
        current_credentials: Option<Arc<Credentials>>,
    }

    impl MemoryCache {
        pub fn new(credentials: Option<Arc<Credentials>>) -> Self {
            MemoryCache {
                current_credentials: credentials,
            }
        }
    }

    impl super::CredentialsCache for MemoryCache {
        fn request_credentials(&self) -> BoxFuture<crate::Result<Option<Arc<Credentials>>>> {
            async { Ok(self.current_credentials.clone()) }.boxed()
        }

        fn persist_credentials(&mut self, credentials: Arc<Credentials>) -> BoxFuture<crate::Result<()>> {
            async move {
                self.current_credentials = Some(credentials.into());
                Ok(())
            }
            .boxed()
        }
    }

    pub struct JsonFileCache {
        mem_cache: MemoryCache,
        cache_path: PathBuf,
    }

    impl JsonFileCache {
        pub fn from_file(file: impl Into<PathBuf>) -> crate::Result<Self> {
            let path = file.into();
            let credentials = if let Ok(contents) = std::fs::read(&path) {
                Some(Arc::new(serde_json::from_slice(&contents)?))
            } else {
                None
            };

            Self::from_credentials(path, credentials)
        }

        pub fn from_credentials(
            file: impl Into<PathBuf>,
            credentials: Option<Arc<Credentials>>,
        ) -> crate::Result<Self> {
            Ok(Self {
                mem_cache: MemoryCache::new(credentials),
                cache_path: file.into(),
            })
        }
    }

    impl super::CredentialsCache for JsonFileCache {
        fn request_credentials(&self) -> BoxFuture<crate::Result<Option<Arc<Credentials>>>> {
            self.mem_cache.request_credentials()
        }

        fn persist_credentials(&mut self, credentials: Arc<Credentials>) -> BoxFuture<crate::Result<()>> {
            async move {
                if let Some(path) = self.cache_path.parent() {
                    std::fs::create_dir_all(path)?;
                }

                // Persist to file, use std::fs for convenience.
                let mut file = BufWriter::new(std::fs::File::create(&self.cache_path)?);
                serde_json::to_writer(&mut file, credentials.as_ref())?;

                self.mem_cache.persist_credentials(credentials).await
            }
            .boxed()
        }
    }
}
