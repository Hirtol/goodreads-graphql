use aws_credential_types::cache::{CredentialsCache, ProvideCachedCredentials, SharedCredentialsCache};
use aws_credential_types::provider::future::ProvideCredentials;
use aws_credential_types::provider::SharedCredentialsProvider;
use aws_credential_types::Credentials;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

#[derive(Debug)]
pub struct GoodreadsCredentialsCache {
    internal: SharedCredentialsCache,
    // Unfortunately necessary to allow for sync access to the credentials. The Lazy cache is only async.
    stored: Arc<RwLock<Option<Credentials>>>,
}

impl GoodreadsCredentialsCache {
    pub fn new(
        provider: SharedCredentialsProvider,
        initial_credentials: Option<Credentials>,
    ) -> GoodreadsCredentialsCache {
        Self {
            internal: CredentialsCache::lazy().create_cache(provider),
            stored: Arc::new(RwLock::new(initial_credentials)),
        }
    }

    pub fn stored_credentials(&self) -> Option<Credentials> {
        self.stored.read().unwrap().clone()
    }
}

impl ProvideCachedCredentials for GoodreadsCredentialsCache {
    fn provide_cached_credentials<'a>(&'a self) -> ProvideCredentials<'a>
    where
        Self: 'a,
    {
        // Check for stored credentials first
        ProvideCredentials::new(async {
            let creds = match self.stored.read().unwrap().as_ref() {
                Some(creds) if creds.expiry().map(|expire| SystemTime::now() < expire).unwrap_or(false) => {
                    Some(creds.clone())
                }
                _ => None,
            };

            if let Some(creds) = creds {
                Ok(creds)
            } else {
                let new_creds = self.internal.provide_cached_credentials().await?;
                (*self.stored.write().unwrap()) = Some(new_creds.clone());
                Ok(new_creds)
            }
        })
    }
}
