# Goodreads GraphQL

A small wrapper to allow easy calls to the Goodreads GraphQL API using temporary anonymous credentials.

## Usage
This is a very basic wrapper, and is best used in combination with a different Rust GraphQL client library like [cynic](https://github.com/obmarg/cynic) or [graphql-client](https://github.com/graphql-rust/graphql-client).

Example using the GraphQL constructs present in this crate:
```rust
use crate::{GoodreadsClient, graphql::GraphQLCustomRequest};
const BOOK_QUERY: &str = r#"
    query getBookByLegacyId($id: Int!) {
        getBookByLegacyId(legacyId: $id) {
            title
        }
    }"#;

async fn get_book_title(legacy_id: u32) -> String {
    let client = GoodreadsClient::default();
    let request =
        GraphQLCustomRequest::from_query(BOOK_QUERY, "getBookByLegacyId").with_variable("id", legacy_id);
    let response: serde_json::Value = client
        .send_graphql_query(request)
        .await
        .expect("Failed request");

    response["data"]["getBookByLegacyId"]["title"]
        .as_str()
        .unwrap()
        .to_string()
}
```