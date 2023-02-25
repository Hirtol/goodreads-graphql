use crate::test_client::credentials::{construct_global_credentials, save_credentials};
use aws_credential_types::Credentials;
use goodreads_graphql::GoodreadsClient;
use once_cell::sync::Lazy;
use std::ops::Deref;

// Force a new thread to spawn to ensure we don't panic due to duplicate runtimes.
static GLOBAL_CREDENTIALS: Lazy<Option<Credentials>> =
    Lazy::new(|| std::thread::spawn(construct_global_credentials).join().unwrap());

pub struct TestClient {
    client: GoodreadsClient,
}

impl TestClient {
    pub fn new() -> Self {
        let client = GoodreadsClient::builder()
            .credentials(GLOBAL_CREDENTIALS.clone())
            .build()
            .expect("Couldn't construct test client");
        Self { client }
    }
}

impl Drop for TestClient {
    fn drop(&mut self) {
        match self.stored_credentials() {
            Some(creds) if creds.expiry() > GLOBAL_CREDENTIALS.as_ref().and_then(|c| c.expiry()) => {
                println!("Test client had more recent credentials, updating...");
                save_credentials(creds);
            }
            _ => {}
        }
    }
}

impl Deref for TestClient {
    type Target = GoodreadsClient;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

mod credentials {
    use aws_credential_types::provider::ProvideCredentials;
    use aws_credential_types::Credentials;
    use aws_types::region::Region;
    use std::path::PathBuf;
    use std::time::SystemTime;

    const CRED_FILE: &str = "cert.json";

    /// One can quickly get API banned by requesting too many credentials, so it's critical to only query for new credentials
    /// when we *have* to.
    ///
    /// In this case we first check a filesystem cache, and only if that one is invalid request a new set of credentials.
    pub fn construct_global_credentials() -> Option<Credentials> {
        let existing_credentials = get_credentials();

        match existing_credentials {
            Some(creds) if Some(SystemTime::now()) < creds.expires_after => Some(creds.into()),
            _ => {
                let provider = goodreads_graphql::identity::GoodreadsCredentialsProvider::new(Region::new("us-east-1"));
                println!("Querying Goodreads API for credentials...");

                let creds = tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(provider.provide_credentials())
                    .ok()?;
                save_credentials(creds.clone());

                Some(creds)
            }
        }
    }

    pub fn save_credentials(credentials: Credentials) -> Option<()> {
        let test_creds: TestCredentials = credentials.into();
        let json = serde_json::to_string(&test_creds).ok()?;
        std::fs::create_dir_all(get_cred_dir()).ok()?;
        std::fs::write(get_cred_path(), json).ok()
    }

    fn get_credentials() -> Option<TestCredentials> {
        let data = std::fs::read_to_string(get_cred_path()).ok()?;

        serde_json::from_str(&data).ok()
    }

    fn get_cred_path() -> PathBuf {
        get_cred_dir().join(CRED_FILE)
    }

    fn get_cred_dir() -> PathBuf {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        root.join("tests").join("resources")
    }

    #[derive(serde::Serialize, serde::Deserialize, Clone, Eq, PartialEq)]
    struct TestCredentials {
        access_key_id: String,
        secret_access_key: String,
        session_token: Option<String>,
        expires_after: Option<SystemTime>,
    }

    impl From<TestCredentials> for Credentials {
        fn from(cred: TestCredentials) -> Self {
            Credentials::new(
                cred.access_key_id,
                cred.secret_access_key,
                cred.session_token,
                cred.expires_after,
                goodreads_graphql::identity::GOODREADS_IDENTITY_POOL,
            )
        }
    }

    impl From<Credentials> for TestCredentials {
        fn from(value: Credentials) -> Self {
            TestCredentials {
                access_key_id: value.access_key_id().to_string(),
                secret_access_key: value.secret_access_key().to_string(),
                session_token: value.session_token().map(|s| s.to_string()),
                expires_after: value.expiry(),
            }
        }
    }
}
