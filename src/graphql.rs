use serde_json::Map;
use std::borrow::Cow;

/// A simple structure to send basic GraphQL queries.
///
/// Using something like [cynic](https://github.com/obmarg/cynic) is recommended for actual applications instead of this.
#[derive(serde::Serialize, Debug, Clone)]
pub struct GraphQLCustomRequest<'a> {
    #[serde(rename = "operationName")]
    pub(crate) operation_name: Cow<'a, str>,
    pub(crate) query: Cow<'a, str>,
    pub(crate) variables: Map<String, serde_json::Value>,
}

impl<'a> GraphQLCustomRequest<'a> {
    /// Create a custom request from a query, alongside the primary `operation_name`
    ///
    /// # Example
    ///
    /// ```rust
    /// # use goodreads_graphql::*;
    /// # use goodreads_graphql::graphql::GraphQLCustomRequest;
    ///
    /// let query: &str = r#"query getBookByLegacyId($id: Int!) {
    ///         getBookByLegacyId(legacyId: $id) {
    ///             title
    ///         }
    ///     }
    /// "#;
    ///
    /// # tokio_test::block_on(async move {
    /// let client = GoodreadsClient::default();
    /// let request =
    ///     GraphQLCustomRequest::from_query(query, "getBookByLegacyId").with_variable("id", 61215351);
    /// let response: serde_json::Value = client
    ///     .send_graphql_query(request)
    ///     .await
    ///     .expect("Failed request");
    ///
    /// assert_eq!(
    ///     response["data"]["getBookByLegacyId"]["title"]
    ///         .as_str()
    ///         .unwrap(),
    ///     "The Fellowship of the Ring"
    /// );
    /// # })
    /// ```
    pub fn from_query(query: impl Into<Cow<'a, str>>, operation_name: impl Into<Cow<'a, str>>) -> Self {
        GraphQLCustomRequest {
            operation_name: operation_name.into(),
            query: query.into(),
            variables: Map::new(),
        }
    }

    /// Add a variable to the query, see [from_query](GraphQLCustomRequest::from_query) for an example.
    pub fn with_variable<T: serde::Serialize, S: Into<String>>(mut self, key: S, value: T) -> Self {
        self.variables.insert(key.into(), serde_json::to_value(value).unwrap());

        self
    }
}
