use serde_json::Map;
use std::borrow::Cow;

#[derive(serde::Serialize, Debug, Clone)]
pub struct GraphQLCustomRequest<'a> {
    #[serde(rename = "operationName")]
    pub(crate) operation_name: Cow<'a, str>,
    pub(crate) query: Cow<'a, str>,
    pub(crate) variables: Map<String, serde_json::Value>,
}

impl<'a> GraphQLCustomRequest<'a> {
    pub fn from_query(query: impl Into<Cow<'a, str>>, operation_name: impl Into<Cow<'a, str>>) -> Self {
        GraphQLCustomRequest {
            operation_name: operation_name.into(),
            query: query.into(),
            variables: Map::new(),
        }
    }

    pub fn with_variable<T: serde::Serialize, S: Into<String>>(mut self, key: S, value: T) -> Self {
        self.variables.insert(key.into(), serde_json::to_value(value).unwrap());

        self
    }
}
