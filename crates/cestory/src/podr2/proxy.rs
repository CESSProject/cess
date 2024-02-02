use std::pin::Pin;

use super::Podr2Result;
use crate::expert::{CesealExpertStub, ExternalResourceKind};
use cestory_api::podr2::{
    podr2_api_server::Podr2Api, podr2_verifier_api_server::Podr2VerifierApi, EchoMessage, RequestBatchVerify,
    RequestGenTag, ResponseBatchVerify, ResponseGenTag,
};
use tokio_stream::Stream;
use tonic::{Request, Response, Status, Streaming};
pub(crate) type ResponseStream = Pin<Box<dyn Stream<Item = Result<ResponseGenTag, Status>> + Send>>;

pub struct Podr2ApiServerProxy<Svc> {
    pub(crate) inner: Svc,
    pub(crate) ceseal_expert: CesealExpertStub,
}

#[tonic::async_trait]
impl<Svc> Podr2Api for Podr2ApiServerProxy<Svc>
where
    Svc: Podr2Api,
{
    #[allow(non_camel_case_types)]
    type request_gen_tagStream = ResponseStream;
    async fn request_gen_tag(
        &self,
        request: Request<Streaming<RequestGenTag>>,
    ) -> Podr2Result<Self::request_gen_tagStream> {
        let _permit = self
            .ceseal_expert
            .try_acquire_permit(ExternalResourceKind::Pord2Service)
            .await?;
        match self.inner.request_gen_tag(request).await {
            Ok(result) => {
                let a = result.into_inner();
                Ok(Response::new(Box::pin(a) as Self::request_gen_tagStream))
            },
            Err(fail) => Err(fail),
        }
    }

    async fn echo(&self, request: Request<EchoMessage>) -> Podr2Result<EchoMessage> {
        self.inner.echo(request).await
    }
}

pub struct Podr2VerifierApiServerProxy<Svc> {
    pub(crate) inner: Svc,
    pub(crate) ceseal_expert: CesealExpertStub,
}

#[tonic::async_trait]
impl<Svc> Podr2VerifierApi for Podr2VerifierApiServerProxy<Svc>
where
    Svc: Podr2VerifierApi,
{
    async fn request_batch_verify(&self, request: Request<RequestBatchVerify>) -> Podr2Result<ResponseBatchVerify> {
        let _permit = self
            .ceseal_expert
            .try_acquire_permit(ExternalResourceKind::Pord2Service)
            .await?;
        self.inner.request_batch_verify(request).await
    }
}
