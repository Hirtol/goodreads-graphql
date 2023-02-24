use aws_credential_types::provider::error::CredentialsError;
use aws_credential_types::provider::ProvideCredentials;
use aws_types::region::Region;
use std::time::SystemTime;

pub const GOODREADS_IDENTITY_POOL: &str = "us-east-1:16da77fa-4392-4d35-bd47-bb0e2d3f73be";

#[derive(Debug)]
pub struct GoodreadsCredentialsProvider {
    client: aws_sdk_cognitoidentity::Client,
}

impl GoodreadsCredentialsProvider {
    /// Create the credentials provider used to configure a [SdkConfig](aws_config::SdkConfig).
    ///
    /// It will acquire an anonymous identity id and subsequent credentials.
    pub fn new() -> GoodreadsCredentialsProvider {
        let config = aws_config::SdkConfig::builder()
            .region(Region::new("us-east-1"))
            .build();

        GoodreadsCredentialsProvider {
            client: aws_sdk_cognitoidentity::Client::new(&config),
        }
    }
}

impl ProvideCredentials for GoodreadsCredentialsProvider {
    fn provide_credentials<'a>(
        &'a self,
    ) -> aws_credential_types::provider::future::ProvideCredentials<'a>
    where
        Self: 'a,
    {
        aws_credential_types::provider::future::ProvideCredentials::new(async {
            let response = self
                .client
                .get_id()
                .identity_pool_id(GOODREADS_IDENTITY_POOL)
                .send()
                .await
                .map_err(CredentialsError::provider_error)?;

            tracing::debug!(
                ?response,
                "Retrieved ID for GoodReads anonymous credentials"
            );

            let cred_result =
                self.client
                    .get_credentials_for_identity()
                    .identity_id(response.identity_id().ok_or_else(|| {
                        CredentialsError::not_loaded("No identity ID was provided")
                    })?)
                    .send()
                    .await
                    .map_err(CredentialsError::provider_error)?;

            let credentials_model = cred_result
                .credentials()
                .ok_or_else(|| CredentialsError::not_loaded("No credentials were returned"))?;

            let final_credentials = aws_sdk_cognitoidentity::Credentials::new(
                credentials_model
                    .access_key_id()
                    .ok_or_else(|| CredentialsError::not_loaded("No access key id was returned"))?,
                credentials_model
                    .secret_key()
                    .ok_or_else(|| CredentialsError::not_loaded("No secret key was returned"))?,
                credentials_model.session_token().map(|r| r.to_string()),
                credentials_model
                    .expiration()
                    .cloned()
                    .and_then(|r| SystemTime::try_from(r).ok()),
                GOODREADS_IDENTITY_POOL,
            );

            tracing::debug!(?final_credentials, "Acquired new temporary credentials");

            Ok(final_credentials)
        })
    }
}
