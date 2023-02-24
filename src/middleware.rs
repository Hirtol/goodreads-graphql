use aws_http::auth::CredentialsStage;
use aws_sig_auth::middleware::SigV4SigningStage;
use aws_sig_auth::signer::{OperationSigningConfig, SigV4Signer};
use aws_smithy_http::middleware::MapRequest;
use aws_smithy_http::operation::Request;
use aws_smithy_http_tower::map_request::{
    AsyncMapRequestLayer, AsyncMapRequestService, MapRequestLayer, MapRequestService,
};
use aws_types::region::{Region, SigningRegion};
use aws_types::SigningService;

#[derive(Debug, Default)]
pub struct GoodreadsMiddleware;

impl<S> tower::Layer<S> for GoodreadsMiddleware {
    type Service = AsyncMapRequestService<
        MapRequestService<MapRequestService<S, SigV4SigningStage>, GoodreadsSigningService>,
        CredentialsStage,
    >;

    fn layer(&self, inner: S) -> Self::Service {
        tower::ServiceBuilder::new()
            .layer(AsyncMapRequestLayer::for_mapper(CredentialsStage::new()))
            .layer(MapRequestLayer::for_mapper(
                GoodreadsSigningService::default(),
            ))
            .layer(MapRequestLayer::for_mapper(SigV4SigningStage::new(
                SigV4Signer::new(),
            )))
            .service(inner)
    }
}

#[derive(Default, Clone, Copy)]
pub struct GoodreadsSigningService;

impl MapRequest for GoodreadsSigningService {
    type Error = anyhow::Error;

    fn name(&self) -> &'static str {
        "goodreads_signing_information"
    }

    fn apply(&self, mut request: Request) -> Result<Request, Self::Error> {
        request
            .properties_mut()
            .insert(SigningRegion::from(Region::new("us-east-1")));
        request
            .properties_mut()
            .insert(SigningService::from_static("appsync"));
        request
            .properties_mut()
            .insert(OperationSigningConfig::default_config());

        Ok(request)
    }
}
