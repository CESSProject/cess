use std::pin::Pin;

use crate::{
    anyhow_to_status as a2s,
    ext_res_permit::{PermitKind, PermitterAware},
    RpcResult,
};
use cestory_api::podr2::{
    podr2_api_server::Podr2Api, podr2_verifier_api_server::Podr2VerifierApi, EchoMessage, RequestAggregateSignature,
    RequestBatchVerify, RequestGenTag, ResponseAggregateSignature, ResponseBatchVerify, ResponseGenTag,
};
use tokio_stream::Stream;
use tonic::{Request, Response, Status, Streaming};
pub(crate) type ResponseStream = Pin<Box<dyn Stream<Item = Result<ResponseGenTag, Status>> + Send>>;

pub struct Podr2ApiServerProxy<Svc> {
    pub(crate) inner: Svc,
}

#[tonic::async_trait]
impl<Svc> Podr2Api for Podr2ApiServerProxy<Svc>
where
    Svc: Podr2Api + PermitterAware,
{
    #[allow(non_camel_case_types)]
    type request_gen_tagStream = ResponseStream;
    async fn request_gen_tag(
        &self,
        request: Request<Streaming<RequestGenTag>>,
    ) -> RpcResult<Self::request_gen_tagStream> {
        let _permit = self
            .inner
            .permitter()
            .try_acquire_permit(PermitKind::Pord2Service)
            .await
            .map_err(a2s)?;
        match self.inner.request_gen_tag(request).await {
            Ok(result) => {
                let a = result.into_inner();
                Ok(Response::new(Box::pin(a) as Self::request_gen_tagStream))
            },
            Err(fail) => Err(fail),
        }
    }

    async fn echo(&self, request: Request<EchoMessage>) -> RpcResult<EchoMessage> {
        self.inner.echo(request).await
    }
}

pub struct Podr2VerifierApiServerProxy<Svc> {
    pub(crate) inner: Svc,
}

#[tonic::async_trait]
impl<Svc> Podr2VerifierApi for Podr2VerifierApiServerProxy<Svc>
where
    Svc: Podr2VerifierApi + PermitterAware,
{
    async fn request_batch_verify(&self, request: Request<RequestBatchVerify>) -> RpcResult<ResponseBatchVerify> {
        let _permit = self
            .inner
            .permitter()
            .try_acquire_permit(PermitKind::Pord2Service)
            .await
            .map_err(a2s)?;
        self.inner.request_batch_verify(request).await
    }
    async fn request_aggregate_signature(
        &self,
        request: Request<RequestAggregateSignature>,
    ) -> RpcResult<ResponseAggregateSignature> {
        self.inner.request_aggregate_signature(request).await
    }
}
