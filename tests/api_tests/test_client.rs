use crate::test_client::credentials::construct_global_credentials;
use goodreads_graphql::credentials::cache::MemoryCache;
use goodreads_graphql::credentials::Credentials;
use goodreads_graphql::GoodreadsClient;
use once_cell::sync::Lazy;
use std::ops::Deref;
use std::sync::Arc;

// Force a new thread to spawn to ensure we don't panic due to duplicate runtimes.
static GLOBAL_CREDENTIALS: Lazy<Option<Arc<Credentials>>> =
    Lazy::new(|| std::thread::spawn(construct_global_credentials).join().unwrap());

pub struct TestClient {
    client: GoodreadsClient,
}

impl TestClient {
    pub fn new() -> Self {
        let client = GoodreadsClient::builder()
            .credentials_cache(Some(MemoryCache::new(GLOBAL_CREDENTIALS.clone())))
            .build()
            .expect("Couldn't construct test client");
        Self { client }
    }
}

impl Deref for TestClient {
    type Target = GoodreadsClient;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

mod credentials {
    use goodreads_graphql::credentials::Credentials;
    use std::path::PathBuf;
    use std::sync::Arc;

    const CRED_FILE: &str = "cert.json";

    /// One can quickly get API banned by requesting too many credentials, so it's critical to only query for new credentials
    /// when we *have* to.
    ///
    /// In this case we first check a filesystem cache, and only if that one is invalid request a new set of credentials.
    pub fn construct_global_credentials() -> Option<Arc<Credentials>> {
        let cache = goodreads_graphql::credentials::cache::JsonFileCache::from_file(get_cred_path());
        let prov = goodreads_graphql::credentials::CredentialsManager::new(cache, None);

        println!("Querying Goodreads API for credentials...");

        let creds = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(prov.credentials())
            .unwrap();

        Some(creds)
    }

    fn get_cred_path() -> PathBuf {
        get_cred_dir().join(CRED_FILE)
    }

    fn get_cred_dir() -> PathBuf {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        root.join("tests").join("resources")
    }
}
