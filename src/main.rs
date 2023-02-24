use crate::client::GoodreadsClient;

mod client;
mod identity;
mod middleware;
mod response_parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let req = aws_smithy_http::body::SdkBody::from(
        r#"{"operationName":"getSearchSuggestions","variables":{"searchQuery":"ghostwater"},"query":"query getSearchSuggestions($searchQuery: String!) {\n  getSearchSuggestions(query: $searchQuery) {\n    edges {\n      ... on SearchBookEdge {\n        node {\n          id\n work {legacyId}\n          title\n description(stripped: true) \n bookSeries{userPosition}\n          primaryContributorEdge {\n            node {\n              name\n              isGrAuthor\n              __typename\n            }\n            __typename\n          }\n          webUrl\n          imageUrl\n          __typename\n        }\n        __typename\n      }\n      __typename\n    }\n    __typename\n  }\n}\n"}
"#,
    );

    // language=graphql
    //     let query = r#"
    //        query {
    //             getSearchSuggestions()
    //        }
    // "#

    let goodreads_client = GoodreadsClient::default();

    let out: serde_json::Value = goodreads_client.send_graphql_query(req).await?;

    println!("Output: {:#?}", out);

    // let client = aws_sdk_appsync::Client::new(&config);
    //
    // let resp = client
    //     .get_graphql_api()
    //     .api_id("kxbwmqov6jgg3daaamb744ycu4")
    //     .send()
    //     .await?;
    // println!("LOL: {:#?}", resp);
    // let pl = s.execute(request.try_into()?).await?;
    Ok(())
}
