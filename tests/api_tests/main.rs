use goodreads_graphql::graphql::GraphQLCustomRequest;
use goodreads_graphql::GoodreadsClient;

#[tokio::test]
pub async fn goodreads_autocomplete_test() {
    let client = GoodreadsClient::default();
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
