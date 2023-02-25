use crate::test_client::TestClient;
use goodreads_graphql::graphql::GraphQLCustomRequest;

mod test_client;

#[tokio::test]
pub async fn goodreads_autocomplete_test() {
    let client = TestClient::new();
    // language=graphql
    let query = r#"
    query getSearchSuggestions($search: String!){
        getSearchSuggestions(query: $search) {
            edges {
                ... on SearchBookEdge {
                    node {
                        title
                    }
                }
            }
        }
    }"#;

    let request = GraphQLCustomRequest::from_query(query, "getSearchSuggestions").with_variable("search", "Unsouled");
    let result: serde_json::Value = client.send_graphql_query(request).await.unwrap();

    assert!(result["data"]["getSearchSuggestions"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["node"]["title"].as_str().unwrap() == "Unsouled"))
}

#[tokio::test]
pub async fn goodreads_direct_book() {
    let client = TestClient::new();
    // language=graphql
    let query = r#"
    query getBookByLegacyId($id: Int!){
        getBookByLegacyId(legacyId: $id) {
            title
        }
    }"#;

    let request = GraphQLCustomRequest::from_query(query, "getBookByLegacyId").with_variable("id", 61215351);
    let result: serde_json::Value = client.send_graphql_query(request).await.unwrap();

    assert_eq!(
        result["data"]["getBookByLegacyId"]["title"].as_str().unwrap(),
        "The Fellowship of the Ring"
    );
}
