use super::PoisResult;
use crate::expert::{CesealExpertStub, ExternalResourceKind};
use cestory_api::pois::{
    pois_certifier_api_server::PoisCertifierApi, pois_verifier_api_server::PoisVerifierApi, Challenge,
    RequestMinerCommitGenChall, RequestMinerInitParam, RequestSpaceProofVerify, RequestSpaceProofVerifyTotal,
    RequestVerifyCommitAndAccProof, RequestVerifyDeletionProof, ResponseMinerInitParam, ResponseSpaceProofVerify,
    ResponseSpaceProofVerifyTotal, ResponseVerifyCommitOrDeletionProof,
};
use tonic::Request;

pub struct PoisCertifierApiServerProxy<Svc> {
    pub(crate) inner: Svc,
    pub(crate) ceseal_expert: CesealExpertStub,
}

#[tonic::async_trait]
impl<Svc> PoisCertifierApi for PoisCertifierApiServerProxy<Svc>
where
    Svc: PoisCertifierApi,
{
    async fn request_miner_get_new_key(
        &self,
        request: Request<RequestMinerInitParam>,
    ) -> PoisResult<ResponseMinerInitParam> {
        self.inner.request_miner_get_new_key(request).await
    }

    async fn request_miner_commit_gen_chall(
        &self,
        request: Request<RequestMinerCommitGenChall>,
    ) -> PoisResult<Challenge> {
        let _permit = self
            .ceseal_expert
            .try_acquire_permit(ExternalResourceKind::PoisCommitGen)
            .await?;
        self.inner.request_miner_commit_gen_chall(request).await
    }

    async fn request_verify_commit_proof(
        &self,
        request: Request<RequestVerifyCommitAndAccProof>,
    ) -> PoisResult<ResponseVerifyCommitOrDeletionProof> {
        let _permit = self
            .ceseal_expert
            .try_acquire_permit(ExternalResourceKind::PoisVerifyCommit)
            .await?;
        self.inner.request_verify_commit_proof(request).await
    }

    async fn request_verify_deletion_proof(
        &self,
        request: Request<RequestVerifyDeletionProof>,
    ) -> PoisResult<ResponseVerifyCommitOrDeletionProof> {
        let _permit = self
            .ceseal_expert
            .try_acquire_permit(ExternalResourceKind::PoisVerifyCommit)
            .await?;
        self.inner.request_verify_deletion_proof(request).await
    }
}

pub struct PoisVerifierApiServerProxy<Svc> {
    pub(crate) inner: Svc,
    pub(crate) ceseal_expert: CesealExpertStub,
}

#[tonic::async_trait]
impl<Svc> PoisVerifierApi for PoisVerifierApiServerProxy<Svc>
where
    Svc: PoisVerifierApi,
{
    async fn request_space_proof_verify_single_block(
        &self,
        request: Request<RequestSpaceProofVerify>,
    ) -> PoisResult<ResponseSpaceProofVerify> {
        let _permit = self
            .ceseal_expert
            .try_acquire_permit(ExternalResourceKind::PoisSpaceProofVerify)
            .await?;
        self.inner.request_space_proof_verify_single_block(request).await
    }

    async fn request_verify_space_total(
        &self,
        request: Request<RequestSpaceProofVerifyTotal>,
    ) -> PoisResult<ResponseSpaceProofVerifyTotal> {
        let _permit = self
            .ceseal_expert
            .try_acquire_permit(ExternalResourceKind::PoisSpaceProofVerify)
            .await?;
        self.inner.request_verify_space_total(request).await
    }
}
