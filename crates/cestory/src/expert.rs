use super::{
    pal::Platform,
    system::WorkerIdentityKey,
    types::{ExpertCmd, ExpertCmdReceiver, ExpertCmdSender, ThreadPoolSafeBox},
    CesealProperties, CesealSafeBox, ChainStorage,
};
use ces_types::{WorkerPublicKey, WorkerRole};
use sp_core::Pair;
use std::{
    borrow::Borrow,
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::sync::{oneshot, Semaphore, SemaphorePermit, TryAcquireError};
use tonic::Status;

#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub enum ExternalResourceKind {
    Pord2Service,
    PoisCommitGen,
    PoisVerifyCommit,
    PoisSpaceProofVerify,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    MpscSend(String),
    #[error("{0}")]
    OneshotRecv(String),
    #[error("no semaphore for external resource type: {0}")]
    NoTypedSemaphore(ExternalResourceKind),
    #[error("acquire semaphore for {0} failed: {1}")]
    SemaphorePermit(ExternalResourceKind, TryAcquireError),
}

impl From<Error> for Status {
    fn from(value: Error) -> Self {
        Status::internal(value.to_string())
    }
}

pub type ExpertCmdResult<T> = Result<T, Error>;

pub async fn run<P: Platform>(ceseal: CesealSafeBox<P>, mut expert_cmd_rx: ExpertCmdReceiver) -> ExpertCmdResult<()> {
    loop {
        match expert_cmd_rx.recv().await {
            Some(cmd) => match cmd {
                ExpertCmd::ChainStorage(resp_sender) => {
                    let guard = ceseal.lock(false, false).expect("ceseal lock failed");
                    let chain_storage = guard.runtime_state.as_ref().map(|s| s.chain_storage());
                    let _ = resp_sender.send(chain_storage);
                },
                ExpertCmd::EgressMessage => {
                    //TODO: We not need this yet
                },
            },
            None => break,
        }
    }
    Ok(())
}

#[derive(Clone)]
pub struct CesealExpertStub {
    ceseal_props: Arc<CesealProperties>,
    cmd_sender: ExpertCmdSender,
    ext_res_semaphores: HashMap<ExternalResourceKind, Arc<Semaphore>>,
    thread_pool: ThreadPoolSafeBox,
}

impl CesealExpertStub {
    pub fn new(ceseal_props: CesealProperties, cmd_sender: ExpertCmdSender) -> Self {
        let thread_pool_cap = ceseal_props.cores.saturating_sub(1).max(1);
        let thread_pool = threadpool::ThreadPool::new(thread_pool_cap as usize);
        info!("PODR2 compute thread pool capacity: {}", thread_pool.max_count());
        let role = ceseal_props.role.clone();
        Self {
            ceseal_props: Arc::new(ceseal_props),
            cmd_sender,
            ext_res_semaphores: make_resource_quotas_by_role(role),
            thread_pool: Arc::new(Mutex::new(thread_pool)),
        }
    }

    pub fn identity_key(&self) -> &WorkerIdentityKey {
        &self.ceseal_props.identity_key
    }

    pub fn identify_public_key(&self) -> WorkerPublicKey {
        self.ceseal_props.identity_key.public()
    }

    pub fn ceseal_props(&self) -> &CesealProperties {
        &self.ceseal_props
    }

    pub fn role(&self) -> &WorkerRole {
        &self.ceseal_props.role
    }

    pub fn thread_pool(&self) -> ThreadPoolSafeBox {
        self.thread_pool.clone()
    }

    pub async fn using_chain_storage<'a, F, R>(&self, call: F) -> ExpertCmdResult<R>
    where
        F: FnOnce(Option<&ChainStorage>) -> R,
    {
        let (tx, rx) = oneshot::channel();
        self.cmd_sender
            .send(ExpertCmd::ChainStorage(tx))
            .await
            .map_err(|e| Error::MpscSend(e.to_string()))?;
        let opt = rx.await.map_err(|e| Error::OneshotRecv(e.to_string()))?;
        Ok(match opt {
            Some(chain_storage) => {
                let chain_storage = chain_storage.read();
                call(Some(chain_storage.borrow()))
            },
            None => call(None),
        })
    }

    pub async fn try_acquire_permit(&self, ext_res_kind: ExternalResourceKind) -> Result<SemaphorePermit, Error> {
        let Some(semaphore) = self.ext_res_semaphores.get(&ext_res_kind) else {
            return Err(Error::NoTypedSemaphore(ext_res_kind))
        };
        trace!("{} semaphore available permits before acquire: {}", ext_res_kind, semaphore.available_permits());
        let s = semaphore.try_acquire().map_err(|e| Error::SemaphorePermit(ext_res_kind, e))?;
        Ok(s)
    }
}

fn make_resource_quotas_by_role(role: WorkerRole) -> HashMap<ExternalResourceKind, Arc<Semaphore>> {
    let semaphore_map: HashMap<ExternalResourceKind, Arc<Semaphore>>;
    use ExternalResourceKind::*;
    //FIXME: the constants below that reference Kaleido code
    match role {
        WorkerRole::Full =>
            semaphore_map = HashMap::from([
                (Pord2Service, Arc::new(Semaphore::new(1))),
                (PoisCommitGen, Arc::new(Semaphore::new(12))),
                (PoisVerifyCommit, Arc::new(Semaphore::new(10))),
                (PoisSpaceProofVerify, Arc::new(Semaphore::new(200))),
            ]),
        WorkerRole::Marker =>
            semaphore_map = HashMap::from([
                (PoisCommitGen, Arc::new(Semaphore::new(100))),
                (PoisVerifyCommit, Arc::new(Semaphore::new(100))),
            ]),
        WorkerRole::Verifier => semaphore_map = HashMap::from([(PoisSpaceProofVerify, Arc::new(Semaphore::new(1000)))]),
    }
    semaphore_map
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        new_sr25519_key,
        system::{CesealMasterKey, WorkerIdentityKey},
    };
    use rsa::{rand_core::OsRng, RsaPrivateKey};
    use tokio::sync::mpsc;
    use ExternalResourceKind::*;

    fn any_rsa_private_key() -> RsaPrivateKey {
        let mut rng = OsRng;
        RsaPrivateKey::new(&mut rng, 1024).unwrap()
    }

    fn any_identity_key() -> WorkerIdentityKey {
        WorkerIdentityKey(new_sr25519_key())
    }

    #[tokio::test]
    async fn permit_acquire_ok_on_full_role() {
        let ceseal_props = CesealProperties {
            role: WorkerRole::Full,
            master_key: CesealMasterKey { rsa_priv_key: any_rsa_private_key(), sr25519_keypair: new_sr25519_key() },
            identity_key: any_identity_key(),
            cores: 2,
        };
        let (tx, _) = mpsc::channel(1);
        let expert = CesealExpertStub::new(ceseal_props, tx);
        {
            let p = expert.try_acquire_permit(Pord2Service).await;
            assert_eq!(p.is_ok(), true);
            let p = expert.try_acquire_permit(Pord2Service).await;
            assert_eq!(p.is_err(), true);
        }
        let p = expert.try_acquire_permit(Pord2Service).await;
        assert_eq!(p.is_ok(), true);
    }
}
