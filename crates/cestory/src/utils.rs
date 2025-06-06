pub mod rpc {
    use tonic::{Response, Status};
    pub type RpcResult<T> = std::result::Result<Response<T>, Status>;

    pub trait RpcStatusSource {
        fn as_status(&self) -> Status;
    }

    impl<T: std::fmt::Debug> RpcStatusSource for T {
        fn as_status(&self) -> Status {
            Status::internal(format!("{:?}", self))
        }
    }

    pub fn as_status<T: RpcStatusSource>(err: T) -> Status {
        err.as_status()
    }

    pub fn anyhow_to_status(err: anyhow::Error) -> Status {
        Status::internal(err.to_string())
    }
}

pub mod cqh {
    use anyhow::{anyhow, Result};
    use cestory_api::chain_client::{
        runtime::{self, runtime_types::pallet_sminer::types::MinerInfo},
        AccountId, BlockNumber, CesChainClient,
    };
    use subxt::config::substrate::H256;

    #[derive(Clone)]
    pub struct ChainQueryHelper {
        pois_param: crate::PoisParam,
        chain_client: CesChainClient,
    }

    impl ChainQueryHelper {
        pub async fn build(chain_client: CesChainClient) -> Result<Self> {
            let pois_param = Self::get_pois_expender_param(&chain_client)
                .await?
                .map(|(k, n, d)| (k as i64, n as i64, d as i64))
                .ok_or(anyhow!("POIS expender param not found on chain"))?;
            Ok(Self { pois_param, chain_client })
        }

        async fn get_pois_expender_param(chain_client: &CesChainClient) -> Result<Option<(u64, u64, u64)>> {
            let q = runtime::storage().sminer().expenders();
            let r = chain_client.storage().at_latest().await?.fetch(&q).await?;
            Ok(r)
        }

        pub fn pois_param(&self) -> &crate::PoisParam {
            &self.pois_param
        }

        pub async fn get_storage_miner_info(&self, miner_account_id: &AccountId) -> Result<Option<MinerInfo>> {
            let a: subxt::utils::AccountId32 = miner_account_id.clone().into();
            let q = runtime::storage().sminer().miner_items(a);
            let r = self.chain_client.storage().at_latest().await?.fetch(&q).await?;
            Ok(r)
        }

        pub async fn is_storage_miner_registered_ignore_state(&self, miner_account_id: &AccountId) -> Result<bool> {
            self.get_storage_miner_info(miner_account_id).await.map(|r| r.is_some())
        }

        pub(crate) async fn get_ceseal_bin_added_at(&self, runtime_hash: &H256) -> Result<Option<BlockNumber>> {
            let q = runtime::storage().tee_worker().ceseal_bin_added_at(runtime_hash);
            let r = self.chain_client.storage().at_latest().await?.fetch(&q).await?;
            Ok(r)
        }

        pub async fn current_block(&self) -> Result<(BlockNumber, u64)> {
            let block = self.chain_client.blocks().at_latest().await?;
            let q = runtime::storage().timestamp().now();
            let r = self
                .chain_client
                .storage()
                .at(block.reference())
                .fetch(&q)
                .await?
                .ok_or(anyhow!("no timestamp pallet in chain"))?;
            Ok((block.number(), r))
        }
    }
}

pub mod ext_res_permit {
    use anyhow::{anyhow, Error, Result};
    use ces_types::WorkerRole;
    use std::{collections::HashMap, sync::Arc};
    use tokio::sync::{Semaphore, SemaphorePermit};

    pub trait PermitterAware {
        fn permitter(&self) -> &Permitter;
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
    pub enum PermitKind {
        Pord2Service,
        PoisCommitGen,
        PoisVerifyCommit,
        PoisSpaceProofVerify,
    }

    #[derive(Clone, Debug)]
    pub struct Permitter {
        ext_res_semaphores: HashMap<PermitKind, Arc<Semaphore>>,
    }

    impl Permitter {
        pub fn new(worker_role: WorkerRole) -> Self {
            let ext_res_semaphores = Self::make_resource_quotas_by_role(worker_role);
            Self { ext_res_semaphores }
        }

        fn make_resource_quotas_by_role(role: WorkerRole) -> HashMap<PermitKind, Arc<Semaphore>> {
            let semaphore_map: HashMap<PermitKind, Arc<Semaphore>>;
            //FIXME: the constants below that reference Kaleido code
            match role {
                WorkerRole::Full => {
                    semaphore_map = HashMap::from([
                        (PermitKind::Pord2Service, Arc::new(Semaphore::new(1))),
                        (PermitKind::PoisCommitGen, Arc::new(Semaphore::new(12))),
                        (PermitKind::PoisVerifyCommit, Arc::new(Semaphore::new(10))),
                        (PermitKind::PoisSpaceProofVerify, Arc::new(Semaphore::new(200))),
                    ])
                },
                WorkerRole::Marker => {
                    semaphore_map = HashMap::from([
                        (PermitKind::PoisCommitGen, Arc::new(Semaphore::new(100))),
                        (PermitKind::PoisVerifyCommit, Arc::new(Semaphore::new(100))),
                    ])
                },
                WorkerRole::Verifier => {
                    semaphore_map = HashMap::from([(PermitKind::PoisSpaceProofVerify, Arc::new(Semaphore::new(1000)))])
                },
            }
            semaphore_map
        }

        pub async fn try_acquire_permit(&self, ext_res_kind: PermitKind) -> Result<SemaphorePermit> {
            let Some(semaphore) = self.ext_res_semaphores.get(&ext_res_kind) else {
                return Err(anyhow!("no semaphore for external resource type: {:?}", ext_res_kind));
            };
            trace!("{} semaphore available permits before acquire: {}", ext_res_kind, semaphore.available_permits());
            let s = semaphore.try_acquire().map_err(Error::new)?;
            Ok(s)
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;

        #[tokio::test]
        async fn permit_acquire_ok_on_full_role() {
            let permitter = Permitter::new(ces_types::WorkerRole::Full);
            {
                let p = permitter.try_acquire_permit(PermitKind::Pord2Service).await;
                assert_eq!(p.is_ok(), true);
                let p = permitter.try_acquire_permit(PermitKind::Pord2Service).await;
                assert_eq!(p.is_err(), true);
            }
            let p = permitter.try_acquire_permit(PermitKind::Pord2Service).await;
            assert_eq!(p.is_ok(), true);
        }
    }
}
