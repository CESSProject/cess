use super::{pal::Platform, system::WorkerIdentityKey, CesealProperties, CesealSafeBox, ChainStorage};
use ces_types::{WorkerPublicKey, WorkerRole};
use sp_core::Pair;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, oneshot, Semaphore, SemaphorePermit, TryAcquireError};
use tonic::Status;

pub type ExpertCmdSender = mpsc::Sender<Cmd>;
pub type ExpertCmdReceiver = mpsc::Receiver<Cmd>;

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

pub type CmdResult<T> = Result<T, Error>;

pub enum Cmd {
    ChainStorage(oneshot::Sender<Option<Arc<ChainStorage>>>),
    EgressMessage,
}

pub async fn run<P: Platform>(ceseal: CesealSafeBox<P>, mut expert_cmd_rx: ExpertCmdReceiver) -> CmdResult<()> {
    loop {
        match expert_cmd_rx.recv().await {
            Some(cmd) => match cmd {
                Cmd::ChainStorage(resp_sender) => {
                    let guard = ceseal.lock(false, false).expect("ceseal lock failed");
                    let chain_storage = guard.runtime_state.as_ref().map(|s| s.chain_storage.clone());
                    let _ = resp_sender.send(chain_storage);
                },
                Cmd::EgressMessage => {
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
    cmd_sender: mpsc::Sender<Cmd>,
    ext_res_semaphores: HashMap<ExternalResourceKind, Arc<Semaphore>>,
}

impl CesealExpertStub {
    pub fn new(ceseal_props: CesealProperties) -> (Self, ExpertCmdReceiver) {
        let (tx, rx) = mpsc::channel(16);
        let role = ceseal_props.role.clone();
        (
            Self {
                ceseal_props: Arc::new(ceseal_props),
                cmd_sender: tx,
                ext_res_semaphores: make_resource_quotas_by_role(role),
            },
            rx,
        )
    }

    pub fn identity_key(&self) -> &WorkerIdentityKey {
        &self.ceseal_props.identity_key
    }

    pub fn identify_public_key(&self) -> WorkerPublicKey {
        self.ceseal_props.identity_key.public()
    }

    pub fn podr2_key(&self) -> &ces_pdp::Keys {
        &self.ceseal_props.podr2_key
    }

    pub fn role(&self) -> &WorkerRole {
        &self.ceseal_props.role
    }

    pub async fn using_chain_storage<F, R>(&self, call: F) -> CmdResult<R>
    where
        F: FnOnce(Option<Arc<ChainStorage>>) -> R,
    {
        let (tx, rx) = oneshot::channel();
        self.cmd_sender
            .send(Cmd::ChainStorage(tx))
            .await
            .map_err(|e| Error::MpscSend(e.to_string()))?;
        let chain_storage = rx.await.map_err(|e| Error::OneshotRecv(e.to_string()))?;
        Ok(call(chain_storage))
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
    use crate::{new_sr25519_key, system::WorkerIdentityKey};
    use ces_pdp::Keys;
    use rsa::{rand_core::OsRng, RsaPrivateKey, RsaPublicKey};
    use ExternalResourceKind::*;

    fn any_podr2_key() -> Keys {
        let mut rng = OsRng;
        let skey = RsaPrivateKey::new(&mut rng, 1024).unwrap();
        let pkey = RsaPublicKey::from(&skey);
        Keys { skey, pkey }
    }

    fn any_identity_key() -> WorkerIdentityKey {
        WorkerIdentityKey(new_sr25519_key())
    }

    #[tokio::test]
    async fn permit_acquire_ok_on_full_role() {
        let ceseal_props =
            CesealProperties { role: WorkerRole::Full, podr2_key: any_podr2_key(), identity_key: any_identity_key() };
        let (expert, _) = CesealExpertStub::new(ceseal_props);
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
