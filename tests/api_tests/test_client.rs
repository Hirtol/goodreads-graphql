use goodreads_graphql::GoodreadsClient;
use std::ops::Deref;

pub struct TestClient {
    client: GoodreadsClient,
}

impl TestClient {
    pub fn new() -> Self {
        let client = GoodreadsClient::builder()
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
