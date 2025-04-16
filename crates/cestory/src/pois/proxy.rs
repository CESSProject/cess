use crate::{
    anyhow_to_status as a2s,
    ext_res_permit::{PermitKind, PermitterAware},
    RpcResult,
};
use cestory_api::pois::{
    pois_certifier_api_server::PoisCertifierApi, pois_verifier_api_server::PoisVerifierApi, Challenge,
    RequestMinerCommitGenChall, RequestMinerInitParam, RequestSpaceProofVerify, RequestSpaceProofVerifyTotal,
    RequestVerifyCommitAndAccProof, RequestVerifyDeletionProof, ResponseMinerInitParam, ResponseSpaceProofVerify,
    ResponseSpaceProofVerifyTotal, ResponseVerifyCommitOrDeletionProof,
};
use tonic::Request;

pub struct PoisCertifierApiServerProxy<Svc> {
    pub(crate) inner: Svc,
}

#[tonic::async_trait]
impl<Svc> PoisCertifierApi for PoisCertifierApiServerProxy<Svc>
where
    Svc: PoisCertifierApi + PermitterAware,
{
    async fn request_miner_get_new_key(
        &self,
        request: Request<RequestMinerInitParam>,
    ) -> RpcResult<ResponseMinerInitParam> {
        self.inner.request_miner_get_new_key(request).await
    }

    async fn request_miner_commit_gen_chall(
        &self,
        request: Request<RequestMinerCommitGenChall>,
    ) -> RpcResult<Challenge> {
        let _permit = self
            .inner
            .permitter()
            .try_acquire_permit(PermitKind::PoisCommitGen)
            .await
            .map_err(a2s)?;
        self.inner.request_miner_commit_gen_chall(request).await
    }

    async fn request_verify_commit_proof(
        &self,
        request: Request<RequestVerifyCommitAndAccProof>,
    ) -> RpcResult<ResponseVerifyCommitOrDeletionProof> {
        let _permit = self
            .inner
            .permitter()
            .try_acquire_permit(PermitKind::PoisVerifyCommit)
            .await
            .map_err(a2s)?;
        self.inner.request_verify_commit_proof(request).await
    }

    async fn request_verify_deletion_proof(
        &self,
        request: Request<RequestVerifyDeletionProof>,
    ) -> RpcResult<ResponseVerifyCommitOrDeletionProof> {
        let _permit = self
            .inner
            .permitter()
            .try_acquire_permit(PermitKind::PoisVerifyCommit)
            .await
            .map_err(a2s)?;
        self.inner.request_verify_deletion_proof(request).await
    }
}

pub struct PoisVerifierApiServerProxy<Svc> {
    pub(crate) inner: Svc,
}

#[tonic::async_trait]
impl<Svc> PoisVerifierApi for PoisVerifierApiServerProxy<Svc>
where
    Svc: PoisVerifierApi + PermitterAware,
{
    async fn request_space_proof_verify_single_block(
        &self,
        request: Request<RequestSpaceProofVerify>,
    ) -> RpcResult<ResponseSpaceProofVerify> {
        let _permit = self
            .inner
            .permitter()
            .try_acquire_permit(PermitKind::PoisSpaceProofVerify)
            .await
            .map_err(a2s)?;
        self.inner.request_space_proof_verify_single_block(request).await
    }

    async fn request_verify_space_total(
        &self,
        request: Request<RequestSpaceProofVerifyTotal>,
    ) -> RpcResult<ResponseSpaceProofVerifyTotal> {
        let _permit = self
            .inner
            .permitter()
            .try_acquire_permit(PermitKind::PoisSpaceProofVerify)
            .await
            .map_err(a2s)?;
        self.inner.request_verify_space_total(request).await
    }
}
