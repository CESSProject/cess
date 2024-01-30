use super::Podr2Result;
use crate::expert::{CesealExpertStub, ExternalResourceKind};
use cestory_api::podr2::{
    podr2_api_server::Podr2Api, podr2_verifier_api_server::Podr2VerifierApi, EchoMessage, RequestBatchVerify,
    RequestGenTag, ResponseBatchVerify, ResponseGenTag,
};
use tonic::Request;

pub struct Podr2ApiServerProxy<Svc> {
    pub(crate) inner: Svc,
    pub(crate) ceseal_expert: CesealExpertStub,
}

#[tonic::async_trait]
impl<Svc> Podr2Api for Podr2ApiServerProxy<Svc>
where
    Svc: Podr2Api,
{
    async fn request_gen_tag(&self, request: Request<RequestGenTag>) -> Podr2Result<ResponseGenTag> {
        let _permit = self
            .ceseal_expert
            .try_acquire_permit(ExternalResourceKind::Pord2Service)
            .await?;
        self.inner.request_gen_tag(request).await
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
