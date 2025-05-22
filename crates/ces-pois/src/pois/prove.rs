use crate::{
    acc::{
        file_manager::*,
        multi_level_acc::{
            new_muti_level_acc, recovery, AccHandle, MutiLevelAcc, WitnessNode, DEFAULT_ELEMS_NUM, DEFAULT_PATH,
        },
        RsaKey,
    },
    expanders::{
        self,
        generate_expanders::{self, construct_stacked_expanders},
        generate_idle_file::{
            AUX_FILE, CLUSTER_DIR_NAME, COMMIT_FILE, DEFAULT_AUX_SIZE, FILE_NAME, HASH_SIZE, SET_DIR_NAME,
        },
        Node, NodeType,
    },
    tree::{self, get_path_proof, DEFAULT_HASH_SIZE},
    util,
};
use anyhow::{anyhow, bail, Context as AnyhowContext, Result};
use num_bigint_dig::BigUint;
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Arc, vec};
use tokio::{fs, sync::RwLock};

const FILE_SIZE: i64 = HASH_SIZE as i64;
const ACC_PATH: &str = DEFAULT_PATH;
const CHALL_ACC_PATH: &str = "./chall_acc/";
const IDLE_FILE_PATH: &str = "./proofs";
const MAXPROOFTHREAD: i64 = 4;
const MINIFILESIZE: i64 = 1024 * 1024;

pub struct Prover<T: AccHandle> {
    pub rw: Arc<RwLock<ProverBody<T>>>,
}

#[derive(Clone)]
pub struct ProverBody<T: AccHandle> {
    pub expanders: expanders::Expanders,
    pub rear: i64,
    pub front: i64,
    pub space: i64,
    pub set_len: i64,
    pub cluster_size: i64,
    context: Context,
    pub id: Vec<u8>,
    pub chain_state: ChainState<T>,
    pub acc_manager: Option<T>,
    pub config: Config,
}

#[derive(Clone)]
pub struct Config {
    pub file_size: i64,
    pub acc_path: String,
    pub chall_acc_path: String,
    pub idle_file_path: String,
    pub max_proof_thread: i64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            file_size: FILE_SIZE,
            acc_path: ACC_PATH.to_string(),
            chall_acc_path: CHALL_ACC_PATH.to_string(),
            idle_file_path: IDLE_FILE_PATH.to_string(),
            max_proof_thread: MAXPROOFTHREAD,
        }
    }
}

#[derive(Clone)]
struct Context {
    pub commited: i64,
    pub added: i64,
    pub generated: i64,
    pub proofed: i64,
}

#[derive(Clone)]
pub struct ChainState<T: AccHandle> {
    pub acc: Option<T>,
    pub challenging: bool,
    // pub del_ch      :chan struct{},
    pub rear: i64,
    pub front: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct MhtProof {
    pub index: expanders::NodeType,
    pub label: Vec<u8>,
    pub paths: Vec<Vec<u8>>,
    pub locs: Vec<u8>,
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Commits {
    pub file_indexs: Vec<i64>,
    pub roots: Vec<Vec<u8>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct CommitProof {
    pub node: MhtProof,
    pub parents: Vec<MhtProof>,
    pub elders: Vec<MhtProof>,
}

#[derive(Debug, Default)]
pub struct AccProof {
    pub indexs: Vec<i64>,
    pub labels: Vec<Vec<u8>>,
    pub wit_chains: Option<Box<WitnessNode>>,
    pub acc_path: Vec<Vec<u8>>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SpaceProof {
    pub left: i64,
    pub right: i64,
    pub proofs: Vec<Vec<MhtProof>>,
    pub roots: Vec<Vec<u8>>,
    pub wit_chains: Vec<WitnessNode>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct DeletionProof {
    pub roots: Vec<Vec<u8>>,
    pub wit_chain: WitnessNode,
    pub acc_path: Vec<Vec<u8>>,
}

pub async fn new_prover<T: AccHandle>(
    k: i64,
    n: i64,
    d: i64,
    id: Vec<u8>,
    space: i64,
    set_len: i64,
) -> Result<Prover<T>> {
    if k <= 0 || n <= 0 || d <= 0 || space <= 0 || id.len() == 0 {
        return Err(anyhow!("bad params"));
    }

    let prover = ProverBody {
        expanders: construct_stacked_expanders(k, n, d),
        rear: 0,
        front: 0,
        space,
        set_len,
        cluster_size: k,
        id,
        context: Context { commited: 0, added: 0, generated: 0, proofed: 0 },
        chain_state: ChainState { acc: None, challenging: false, rear: 0, front: 0 },
        acc_manager: None,
        config: Config::default(),
    };

    Ok(Prover { rw: Arc::new(RwLock::new(prover)) })
}

impl Prover<MutiLevelAcc> {
    pub async fn init(&mut self, key: RsaKey, config: Config) -> Result<()> {
        if key.g.to_bytes_be().len() == 0 || key.n.to_bytes_be().len() == 0 {
            return Err(anyhow!("bad init params"));
        }
        let mut prover_guard = self.rw.write().await;

        prover_guard.config = config;

        prover_guard.acc_manager = Some(new_muti_level_acc(&prover_guard.config.acc_path, key.clone()).await?);
        let _ = new_muti_level_acc(&prover_guard.config.chall_acc_path, key).await?;
        Ok(())
    }

    pub async fn recovery(&mut self, key: RsaKey, front: i64, rear: i64, config: Config) -> Result<()> {
        {
            let mut prover_guard = self.rw.write().await;
            if key.g.to_bytes_be().len() == 0
                || key.n.to_bytes_be().len() == 0
                || front < 0
                || rear < 0
                || front > rear
                || rear % (prover_guard.set_len * prover_guard.cluster_size) != 0
            {
                bail!("bad recovery params");
            }
            prover_guard.config = config.clone();

            //recovery front and rear
            prover_guard.front = front;
            prover_guard.rear = rear;

            //recovery acc
            prover_guard.acc_manager = Some(recovery(&prover_guard.config.acc_path, key.clone(), front, rear).await?);
        }
        {
            //recovery context
            let mut generated = self.calc_generated_file(&config.idle_file_path).await?;
            let mut prover_guard = self.rw.write().await;
            if generated % (prover_guard.set_len * prover_guard.cluster_size) != 0 {
                //restores must be performed in units of the number of files in a set
                generated -= generated % (prover_guard.set_len * prover_guard.cluster_size)
            };
            prover_guard.context.generated = rear + generated; //generated files do not need to be generated again
            prover_guard.context.added = rear + generated; //the file index to be generated should be consistent with the generated file index firstly
            prover_guard.context.commited = rear;
            prover_guard.space -= (prover_guard.rear - prover_guard.front) * prover_guard.config.file_size; //calc proved space
            prover_guard.space -= generated / prover_guard.cluster_size
                * (prover_guard.cluster_size + prover_guard.expanders.k)
                * prover_guard.config.file_size; //calc generated space
        }
        //backup acc file for challenge
        util::copy_files(&config.acc_path, &config.chall_acc_path)?;

        Ok(())
    }

    pub async fn set_challenge_state(&mut self, key: RsaKey, acc_snp: Vec<u8>, front: i64, rear: i64) -> Result<()> {
        let mut prover_guard = self.rw.write().await;

        prover_guard.chain_state.rear = rear;
        prover_guard.chain_state.front = front;

        let chain_acc = BigUint::from_bytes_be(&acc_snp);

        prover_guard.chain_state.acc = Some(
            recovery(CHALL_ACC_PATH, key, front, rear)
                .await
                .context("recovery chain state error")?,
        );

        let local_acc = BigUint::from_bytes_be(
            &prover_guard
                .chain_state
                .acc
                .as_mut()
                .ok_or_else(|| anyhow!("Accumulator manager is not initialized."))?
                .get_snapshot()
                .await
                .read()
                .await
                .accs
                .as_ref()
                .unwrap()
                .read()
                .await
                .value
                .clone(),
        );

        if chain_acc != local_acc {
            bail!("recovery chain state error:The restored accumulator value is not equal to the snapshot value");
        }

        prover_guard.chain_state.challenging = true;
        // prover_guard.chain_state.del_ch = None;

        Ok(())
    }

    pub async fn set_challenge_state_for_test(&mut self, front: i64, rear: i64) {
        self.rw.write().await.chain_state = ChainState { acc: None, challenging: false, rear, front };
    }

    // GenerateIdleFileSet generate num=(p.setLen*p.clusterSize(==k)) idle files, num must be consistent with the data given by CESS, otherwise it cannot pass the verification
    // This method is not thread-safe, please do not use it concurrently!
    pub async fn generate_idle_file_set(&mut self) -> Result<()> {
        let mut prover_guard = self.rw.write().await;

        let file_num = prover_guard.set_len * prover_guard.cluster_size;
        let idle_file_path = prover_guard.config.idle_file_path.clone();
        let free_space = util::get_dir_free_space(&idle_file_path).context("get free space error")? / 1024 * 1024;
        let reserved = 256_i64;

        if prover_guard.space == file_num * prover_guard.config.file_size
            && free_space > (prover_guard.expanders.k * prover_guard.config.file_size + reserved) as u64
        {
            prover_guard.space += prover_guard.expanders.k * prover_guard.config.file_size;
        }

        if prover_guard.space
            < (file_num + prover_guard.set_len * prover_guard.expanders.k) * prover_guard.config.file_size
        {
            bail!("generate idle file set error: not enough space")
        }

        prover_guard.context.added += file_num;
        prover_guard.space -=
            (file_num + prover_guard.set_len * prover_guard.expanders.k) * prover_guard.config.file_size;
        let start = (prover_guard.context.added - file_num) / prover_guard.cluster_size + 1;

        let id = prover_guard.id.clone();
        let set_len = prover_guard.set_len;
        prover_guard
            .expanders
            .generate_idle_file_set(id, start, set_len, &idle_file_path)
            .await
            .map_err(|e| {
                prover_guard.context.added -= file_num;
                prover_guard.space +=
                    (file_num + prover_guard.set_len * prover_guard.expanders.k) * prover_guard.config.file_size;
                e
            })?;
        prover_guard.context.generated += file_num;

        Ok(())
    }

    pub async fn generate_idle_file_sets(&mut self, t_num: i64) -> Result<()> {
        let mut prover_guard = self.rw.write().await;
        let mut t_num = t_num;

        if t_num <= 0 {
            bail!("generate idle file sets error: bad thread number");
        }

        // Get available space
        let free_space = util::get_dir_free_space(IDLE_FILE_PATH)? / (1024 * 1024); // MB
        let reserved = 256_i64;

        let file_num = prover_guard.set_len * prover_guard.cluster_size;

        if prover_guard.space == file_num * FILE_SIZE
            && free_space > (prover_guard.expanders.k * FILE_SIZE + reserved) as u64
        {
            prover_guard.space += prover_guard.expanders.k * FILE_SIZE;
        }

        if prover_guard.space < (file_num + prover_guard.set_len * prover_guard.expanders.k) * FILE_SIZE * t_num {
            if prover_guard.space >= (file_num + prover_guard.set_len * prover_guard.expanders.k) * FILE_SIZE {
                t_num = 1;
            } else {
                bail!("generate idle file sets error: space is full");
            }
        };

        let curr_index = prover_guard.context.added / prover_guard.cluster_size + 1;
        prover_guard.context.added += file_num * t_num;
        prover_guard.space -= (file_num + prover_guard.set_len * prover_guard.expanders.k) * FILE_SIZE * t_num;

        let mut tasks = Vec::new();

        for i in 0..t_num {
            let start = curr_index + i * prover_guard.set_len;
            let mut prover_guard = prover_guard.clone();

            let task = tokio::task::spawn(async move {
                prover_guard
                    .expanders
                    .generate_idle_file_set(prover_guard.id.clone(), start, prover_guard.set_len, IDLE_FILE_PATH)
                    .await
            });
            tasks.push(task);
        }

        // Wait for all tasks to finish
        for (_, task) in tasks.into_iter().enumerate() {
            task.await.context("spawn taks failed")??;
        }

        prover_guard.context.generated += file_num * t_num;
        Ok(())
    }

    // CommitRollback need to be invoked when submit commits to verifier failure
    pub async fn commit_roll_back(&mut self) -> bool {
        let mut prover_guard = self.rw.write().await;
        prover_guard.context.commited -= prover_guard.set_len * prover_guard.cluster_size;
        true
    }

    // AccRollback need to be invoked when submit or verify acc proof failure,
    // the update of the accumulator is serial and blocking, you need to update or roll back in time.
    pub async fn acc_roll_back(&mut self, is_del: bool) -> bool {
        let mut prover_guard = self.rw.write().await;

        if !is_del {
            prover_guard.context.commited -= prover_guard.set_len * prover_guard.cluster_size;
        };

        prover_guard.acc_manager.as_mut().unwrap().rollback().await
    }

    pub async fn sync_chain_pois_status(&mut self, front: i64, rear: i64) -> Result<()> {
        let mut prover_guard = self.rw.write().await;
        if prover_guard.front == front && prover_guard.rear == rear {
            return Ok(());
        };

        prover_guard.acc_manager = Some(
            recovery(
                ACC_PATH,
                prover_guard
                    .acc_manager
                    .as_mut()
                    .unwrap()
                    .get_snapshot()
                    .await
                    .read()
                    .await
                    .key
                    .clone(),
                front,
                rear,
            )
            .await
            .context("reflash acc error")?,
        );
        //recovery front and rear
        prover_guard.front = front;
        prover_guard.rear = rear;
        prover_guard.context.commited = rear;

        Ok(())
    }

    // UpdateStatus need to be invoked after verify commit proof and acc proof success,
    // the update of the accumulator is serial and blocking, you need to update or roll back in time.
    pub async fn update_status(&mut self, num: i64, is_delete: bool) -> Result<()> {
        if num < 0 {
            bail!("bad files number");
        }

        let mut prover_guard = self.rw.write().await;

        if is_delete {
            prover_guard.front += num;
            prover_guard.acc_manager.as_mut().unwrap().update_snapshot().await;

            let index = (prover_guard.front - 1) / DEFAULT_ELEMS_NUM as i64;
            if let Err(err) = clean_backup(ACC_PATH, index) {
                bail!("delete idle files error: {}", err);
            }
            return Ok(());
        }

        prover_guard.rear += num;
        prover_guard.acc_manager.as_mut().unwrap().update_snapshot().await;
        let rear = prover_guard.rear;
        let front = prover_guard.front;
        drop(prover_guard);

        if let Err(err) = self.organize_files(rear - num, num).await {
            bail!("update prover status error: {}", err);
        }

        for i in ((front + num - 1) / num) * num..rear - num {
            if let Err(err) = self.organize_files(i, num).await {
                bail!("update prover status error: {}", err);
            }
        }
        Ok(())
    }

    pub async fn get_space(&self) -> i64 {
        self.rw.read().await.space
    }

    pub async fn return_space(&mut self, size: i64) {
        self.rw.write().await.space += size;
    }

    pub async fn get_acc_value(&self) -> Vec<u8> {
        let key_len = 256;

        let acc = self
            .rw
            .write()
            .await
            .acc_manager
            .clone()
            .unwrap()
            .get_snapshot()
            .await
            .read()
            .await
            .accs
            .clone()
            .unwrap()
            .read()
            .await
            .value
            .clone();
        let mut res = vec![0_u8; key_len];
        if acc.len() > key_len {
            res.copy_from_slice(&acc[acc.len() - 256..]);
            return res;
        }
        res[256 - acc.len()..].copy_from_slice(&acc);
        return res;
    }

    // GetCount get Count Safely
    pub async fn get_rear(&self) -> i64 {
        self.rw.read().await.rear
    }

    pub async fn get_front(&self) -> i64 {
        self.rw.read().await.front
    }

    pub async fn get_num_of_file_in_set(&self) -> i64 {
        self.rw.read().await.set_len * self.rw.read().await.cluster_size
    }

    pub async fn commit_data_is_ready(&self) -> bool {
        let file_num = self.rw.read().await.context.generated;
        let commited = self.rw.read().await.context.commited;
        return file_num - commited >= self.rw.read().await.set_len * self.rw.read().await.cluster_size;
    }

    pub async fn get_chain_state(&self) -> ChainState<MutiLevelAcc> {
        let prover_guard = self.rw.read().await;
        ChainState {
            rear: prover_guard.chain_state.rear,
            front: prover_guard.chain_state.front,
            acc: prover_guard.chain_state.acc.clone(),
            challenging: false,
        }
    }

    // RestChallengeState must be called when space proof is finished
    pub async fn rest_challenge_state(&mut self) {
        let mut prover_guard = self.rw.write().await;

        prover_guard.context.proofed = 0;
        // prover_guard.chain_state.del_ch = None;
        prover_guard.chain_state.challenging = false;

        if prover_guard.chain_state.front >= DEFAULT_ELEMS_NUM as i64 {
            let index = (prover_guard.chain_state.front - DEFAULT_ELEMS_NUM as i64) / DEFAULT_ELEMS_NUM as i64;
            let _ = delete_acc_data(CHALL_ACC_PATH, index as i32);
        };
    }

    // GetIdleFileSetCommits can not run concurrently! And num must be consistent with the data given by CESS.
    pub async fn get_idle_file_set_commits(&mut self) -> Result<Commits> {
        let mut commits = Commits::default();
        let mut prover_guard = self.rw.write().await;

        let file_num = prover_guard.context.generated;
        let commited = prover_guard.context.commited;
        let commit_num = prover_guard.set_len * prover_guard.cluster_size;

        if file_num - commited < commit_num {
            bail!("get commits error:bad commit data");
        }
        //read commit file of idle file set
        let name = Path::new(IDLE_FILE_PATH)
            .join(format!(
                "{}-{}",
                expanders::generate_idle_file::SET_DIR_NAME,
                (commited) / (prover_guard.set_len * prover_guard.cluster_size) + 1
            ))
            .join(expanders::generate_idle_file::COMMIT_FILE);
        let root_num = commit_num + prover_guard.expanders.k * prover_guard.set_len + 1;
        commits.roots =
            util::read_proof_file(&name, root_num as usize, tree::DEFAULT_HASH_SIZE as usize).map_err(|e| e)?;
        commits.file_indexs = vec![0_i64; commit_num as usize];
        for i in 0..commit_num {
            commits.file_indexs[i as usize] = commited + i + 1;
        }
        prover_guard.context.commited += commit_num;

        Ok(commits)
    }

    pub async fn prove_commit_and_acc(
        &mut self,
        challenges: Vec<Vec<i64>>,
    ) -> Result<(Option<Vec<Vec<CommitProof>>>, Option<AccProof>)> {
        let commit_proofs = self.prove_commits(challenges.clone()).await?;
        let acc_proof = self.prove_acc(challenges).await?;

        //copy new acc data to challenging acc path
        let prover_guard = self.rw.read().await;
        let index = prover_guard.rear / DEFAULT_ELEMS_NUM as i64;
        backup_acc_data_for_chall(ACC_PATH, CHALL_ACC_PATH, index)?;

        Ok((commit_proofs, acc_proof))
    }

    pub async fn prove_acc(&mut self, challenges: Vec<Vec<i64>>) -> Result<Option<AccProof>> {
        let mut prover_guard = self.rw.write().await;
        if challenges.len() != prover_guard.set_len as usize {
            bail!("update acc error:bad challenges data")
        }
        let file_num = prover_guard.set_len * prover_guard.cluster_size;
        let mut labels: Vec<Vec<u8>> = vec![Vec::new(); file_num as usize];
        let mut proof = AccProof::default();
        proof.indexs = vec![0_i64; file_num as usize];
        //read commit roots file
        let fname = Path::new(IDLE_FILE_PATH)
            .join(format!("{}-{}", SET_DIR_NAME, (challenges[0][0] - 1) / prover_guard.set_len + 1))
            .join(COMMIT_FILE);

        let roots = util::read_proof_file(
            &fname,
            ((prover_guard.expanders.k + prover_guard.cluster_size) * prover_guard.set_len + 1) as usize,
            DEFAULT_HASH_SIZE as usize,
        )
        .context("update acc error")?;

        for i in 0..prover_guard.set_len as usize {
            for j in 0..prover_guard.cluster_size as usize {
                let index = (challenges[i][0] - 1) * prover_guard.cluster_size + j as i64 + 1;
                proof.indexs[i * prover_guard.cluster_size as usize + j] = index;
                let root = roots[(prover_guard.expanders.k as usize + j) * prover_guard.set_len as usize + i].clone();
                let mut label = prover_guard.id.clone();
                label.extend_from_slice(&expanders::get_bytes(index));
                label.extend_from_slice(&root);
                labels[i * prover_guard.cluster_size as usize + j] = expanders::generate_idle_file::get_hash(&label);
            }
        }
        let (wit_chains, acc_path) = prover_guard
            .acc_manager
            .as_mut()
            .ok_or_else(|| anyhow!("acc manager is none"))?
            .add_elements_and_proof(labels.clone())
            .await?;
        proof.wit_chains = Some(Box::new(wit_chains));
        proof.acc_path = acc_path;

        proof.labels = labels;

        Ok(Some(proof))
    }

    pub async fn read_file_labels(&self, cluster: i64, fidx: i64, buf: &mut Vec<u8>) -> Result<()> {
        let file_name = Path::new(IDLE_FILE_PATH)
            .join(format!("{}-{}", SET_DIR_NAME, (cluster - 1) / self.rw.read().await.set_len + 1))
            .join(format!("{}-{}", CLUSTER_DIR_NAME, cluster))
            .join(format!("{}-{}", FILE_NAME, fidx + self.rw.read().await.expanders.k));

        util::read_file_to_buf(&file_name, buf).context("read file labels error")
    }

    pub async fn read_aux_data(&self, cluster: i64, fidx: i64, buf: &mut Vec<u8>) -> Result<()> {
        let file_name = Path::new(IDLE_FILE_PATH)
            .join(format!("{}-{}", SET_DIR_NAME, (cluster - 1) / self.rw.read().await.set_len + 1))
            .join(format!("{}-{}", CLUSTER_DIR_NAME, cluster))
            .join(format!("{}-{}", AUX_FILE, fidx + self.rw.read().await.expanders.k));

        util::read_file_to_buf(&file_name, buf).context("read aux data error")
    }

    // ProveCommit prove commits no more than MaxCommitProofThread
    pub async fn prove_commits(&self, challenges: Vec<Vec<i64>>) -> Result<Option<Vec<Vec<CommitProof>>>> {
        let lens = challenges.len();
        let mut proof_set: Vec<Vec<CommitProof>> = vec![Vec::new(); lens];
        let prover_guard = self.rw.read().await;
        for i in 0..lens {
            let mut proofs: Vec<CommitProof> = vec![CommitProof::default(); challenges[i].len() - 1];
            let fdir = Path::new(IDLE_FILE_PATH)
                .join(format!(
                    "{}-{}",
                    expanders::generate_idle_file::SET_DIR_NAME,
                    (challenges[i][0] - 1) / prover_guard.set_len + 1
                ))
                .join(format!("{}-{}", expanders::generate_idle_file::CLUSTER_DIR_NAME, challenges[i][0]));
            for j in 1..(proofs.len() + 1) as i64 {
                let mut index = challenges[i][j as usize];
                if j > prover_guard.cluster_size + 1 {
                    index = proofs[j as usize - 2].parents[challenges[i][j as usize] as usize].index as i64;
                }
                let mut layer = index / prover_guard.expanders.n;
                if j < prover_guard.cluster_size + 1 {
                    layer = prover_guard.expanders.k + j - 1;
                }
                let neighbor = if layer != 0 || i != 0 {
                    let mut cid = challenges[i][0] - 1;
                    if cid % prover_guard.set_len == 0 {
                        cid += prover_guard.set_len;
                    }
                    let path_buf = Path::new(IDLE_FILE_PATH)
                        .join(format!(
                            "{}-{}",
                            expanders::generate_idle_file::SET_DIR_NAME,
                            (challenges[i][0] - 1) / prover_guard.set_len + 1,
                        ))
                        .join(format!("{}-{}", expanders::generate_idle_file::CLUSTER_DIR_NAME, cid))
                        .join(format!(
                            "{}-{}",
                            expanders::generate_idle_file::FILE_NAME,
                            layer - (prover_guard.set_len - i as i64) / prover_guard.set_len
                        ));
                    path_buf
                } else {
                    Path::new(IDLE_FILE_PATH).to_path_buf()
                };
                proofs[j as usize - 1] = self
                    .generate_commit_proof(&fdir, neighbor.as_path(), challenges[i][0], index, layer)
                    .await?;
            }
            proof_set[i] = proofs;
        }
        Ok(Some(proof_set))
    }

    pub async fn generate_path_proof(
        &self,
        mht: &mut tree::LightMHT,
        data: &mut [u8],
        index: i64,
        node_idx: i64,
    ) -> Result<MhtProof> {
        tree::calc_light_mht_with_bytes(mht, data, HASH_SIZE as i64);
        let path_proof =
            tree::get_path_proof(&mht, data, index, HASH_SIZE as i64, false).context("generate path proof error")?;

        let mut label: Vec<u8> = vec![0u8; HASH_SIZE as usize];
        label.copy_from_slice(&data[index as usize * HASH_SIZE as usize..(index + 1) as usize * HASH_SIZE as usize]);

        Ok(MhtProof { index: node_idx as NodeType, label, paths: path_proof.path, locs: path_proof.locs })
    }

    pub async fn get_path_proof_with_aux(
        &self,
        aux: &mut Vec<u8>,
        data: &mut Vec<u8>,
        index: i64,
        node_idx: i64,
    ) -> Result<MhtProof> {
        let path_proof = tree::get_path_proof_with_aux(data, aux, index as usize, HASH_SIZE as usize)?;

        let mut label = vec![0u8; HASH_SIZE as usize];
        label.copy_from_slice(&data[index as usize * HASH_SIZE as usize..(index + 1) as usize * HASH_SIZE as usize]);

        Ok(MhtProof { index: node_idx as NodeType, label, paths: path_proof.path, locs: path_proof.locs })
    }

    pub async fn generate_commit_proof(
        &self,
        fdir: &Path,
        neighbor: &Path,
        count: i64,
        c: i64,
        mut subfile: i64,
    ) -> Result<CommitProof> {
        let prover_guard = self.rw.read().await;
        if subfile < 0 || subfile > prover_guard.cluster_size + prover_guard.expanders.k - 1 {
            bail!("generate commit proof error: bad node index")
        }
        let mut data = prover_guard.expanders.file_pool.clone();
        let fpath = fdir.join(format!("{}-{}", expanders::generate_idle_file::FILE_NAME, subfile));

        util::read_file_to_buf(&fpath, &mut data).context("generate commit proof error")?;

        let mut node_tree = tree::get_light_mht(prover_guard.expanders.n);
        let mut parent_tree = tree::get_light_mht(prover_guard.expanders.n);
        let index = c % prover_guard.expanders.n;

        let path_proof = self
            .generate_path_proof(&mut node_tree, &mut data, index, c)
            .await
            .context("generate commit proof error")?;

        let mut proof = CommitProof::default();
        proof.node = path_proof;

        let mut pdata = prover_guard.expanders.file_pool.clone();

        let mut aux: Vec<u8> = vec![0u8; DEFAULT_AUX_SIZE as usize * DEFAULT_HASH_SIZE as usize];

        //add neighbor node dependency
        proof.elders =
            vec![MhtProof::default(); (subfile / prover_guard.expanders.k * prover_guard.expanders.k / 2) as usize + 1];

        if !neighbor.eq(Path::new("")) {
            util::read_file_to_buf(&neighbor, &mut pdata).context("generate commit proof error")?;

            util::read_file_to_buf(
                Path::new(&neighbor.to_str().unwrap_or("").replacen(FILE_NAME, AUX_FILE, 1)),
                &mut aux,
            )
            .context("generate commit proof error")?;
            proof.elders[0] = self.get_path_proof_with_aux(&mut aux, &mut pdata, index, index).await?;
        }

        if subfile == 0 {
            return Ok(proof);
        }

        //file remapping
        let layer = subfile;
        if subfile >= prover_guard.expanders.k {
            let base_layer = (subfile - prover_guard.expanders.k / 2) / prover_guard.expanders.k;
            subfile = prover_guard.expanders.k;

            //add elder node dependency
            for i in 0..prover_guard.expanders.k / 2 {
                let f_path = fdir.join(format!("{}-{}", FILE_NAME, base_layer + i * 2));
                let a_path = fdir.join(format!("{}-{}", AUX_FILE, base_layer + i * 2));

                util::read_file_to_buf(&f_path, &mut pdata).context("generate commit proof error")?;
                util::read_file_to_buf(&a_path, &mut aux).context("generate commit proof error")?;
                proof.elders[i as usize + 1] = self
                    .get_path_proof_with_aux(
                        &mut aux,
                        &mut pdata,
                        index,
                        index + (base_layer + i * 2) * prover_guard.expanders.n,
                    )
                    .await
                    .context("generate commit proof error")?;
            }
        }

        let mut node = Node::new(c as NodeType);
        node.parents = Vec::with_capacity(prover_guard.expanders.d as usize + 1);
        generate_expanders::calc_parents(&prover_guard.expanders, &mut node, &prover_guard.id, count, layer);

        let fpath = fdir.join(format!("{}-{}", FILE_NAME, subfile - 1));

        util::read_file_to_buf(&fpath, &mut pdata).context("generate commit proof error")?;

        tree::calc_light_mht_with_bytes(&mut parent_tree, &mut pdata, HASH_SIZE as i64);
        let lens = node.parents.len();
        let mut parent_proofs = vec![MhtProof::default(); lens];

        for i in 0..lens {
            let index = node.parents[i] as usize % prover_guard.expanders.n as usize;
            let mut label = vec![0u8; HASH_SIZE as usize];

            let mut path_proof = if node.parents[i] as i64 >= subfile * prover_guard.expanders.n {
                label.copy_from_slice(&data[index * HASH_SIZE as usize..(index + 1) * HASH_SIZE as usize]);
                get_path_proof(&node_tree, &data, index as i64, HASH_SIZE as i64, false)?
            } else {
                label.copy_from_slice(&pdata[index * HASH_SIZE as usize..(index + 1) * HASH_SIZE as usize]);
                get_path_proof(&parent_tree, &pdata, index as i64, HASH_SIZE as i64, false)?
            };
            if node.parents[i] % 6 != 0 {
                path_proof.path = Vec::new();
                path_proof.locs = Vec::new();
            }
            parent_proofs[i] = MhtProof { index: node.parents[i], label, paths: path_proof.path, locs: path_proof.locs }
        }

        proof.parents = parent_proofs;
        Ok(proof)
    }

    pub async fn prove_space(&self, challenges: Vec<i64>, left: i64, right: i64) -> Result<Arc<RwLock<SpaceProof>>> {
        if challenges.is_empty()
            || right - left <= 0
            || left <= self.rw.read().await.chain_state.front
            || right > self.rw.read().await.chain_state.rear + 1
        {
            bail!("prove space error:bad challenge range");
        }

        let proof = Arc::new(RwLock::new(SpaceProof {
            proofs: vec![vec![]; (right - left) as usize],
            roots: vec![vec![]; (right - left) as usize],
            wit_chains: vec![WitnessNode::default(); (right - left) as usize],
            left,
            right,
        }));

        let indexs = Arc::new(RwLock::new(vec![0; (right - left) as usize]));
        let mut threads = MAXPROOFTHREAD;
        if right - left < threads {
            threads = right - left;
        }
        if threads <= 0 {
            threads = 2;
        }
        let block = ((right - left) / threads) * threads;

        let mut tasks = Vec::new();
        for i in 0..threads {
            let gl = left + i * block;
            let mut gr = left + (i + 1) * block;
            if gr > right {
                gr = right;
            }

            let p = self.rw.clone();
            let proof_clone = proof.clone();
            let challenges = challenges.clone();
            let indexs = indexs.clone();

            let task = tokio::spawn(async move {
                let p = Prover { rw: p };
                for fidx in gl..gr {
                    let mut data = p.rw.read().await.expanders.file_pool.clone();

                    if let Err(e) = p
                        .read_file_labels(
                            ((fidx - 1) / p.rw.read().await.cluster_size) + 1,
                            (fidx - 1) % p.rw.read().await.cluster_size,
                            &mut data,
                        )
                        .await
                    {
                        return Err(e);
                    }

                    let mut aux = vec![0u8; DEFAULT_AUX_SIZE as usize * tree::DEFAULT_HASH_SIZE as usize];
                    if let Err(e) = p
                        .read_aux_data(
                            ((fidx - 1) / p.rw.read().await.cluster_size) + 1,
                            (fidx - 1) % p.rw.read().await.cluster_size,
                            &mut aux,
                        )
                        .await
                    {
                        return Err(e);
                    }

                    let mut mht = tree::get_light_mht(DEFAULT_AUX_SIZE);
                    tree::calc_light_mht_with_aux(&mut mht, &aux);

                    indexs.write().await[(fidx - left) as usize] = fidx;

                    let mut proof_guard = proof_clone.write().await;
                    proof_guard.roots[(fidx - left) as usize] = tree::get_root(&mht);
                    proof_guard.proofs[(fidx - left) as usize] = vec![];

                    for (_, challenge) in challenges.clone().into_iter().enumerate() {
                        let idx = (challenge % p.rw.read().await.expanders.n) as usize;

                        let path_proof = tree::get_path_proof_with_aux(&data, &mut aux, idx, HASH_SIZE as usize)?;
                        let mut label = vec![0u8; HASH_SIZE as usize];
                        label.copy_from_slice(&data[idx * HASH_SIZE as usize..(idx + 1) * HASH_SIZE as usize]);
                        proof_guard.proofs[(fidx - left) as usize].push(MhtProof {
                            paths: path_proof.path,
                            locs: path_proof.locs,
                            index: challenge as expanders::NodeType,
                            label,
                        });
                    }
                }
                Ok(())
            });
            tasks.push(task);
        }
        for (i, task) in tasks.into_iter().enumerate() {
            task.await
                .context("prove space error")
                .map(|e| anyhow!("prove space task index {} error: {:?}", i, e))?;
        }

        proof.write().await.wit_chains = self
            .rw
            .write()
            .await
            .chain_state
            .acc
            .as_mut()
            .unwrap()
            .get_witness_chains(indexs.read().await.clone())
            .await
            .context("prove space error")?;
        self.rw.write().await.context.proofed = right - 1;

        Ok(proof)
    }

    // ProveDeletion sort out num*IdleFileSize(unit MiB) available space,
    // you need to update prover status with this value rather than num after the verification is successful.
    pub async fn prove_deletion(&mut self, num: i64) -> Result<DeletionProof> {
        if num <= 0 {
            bail!("prove deletion error: bad file number");
        }

        if self.rw.read().await.rear - self.rw.read().await.front < num {
            bail!("prove deletion error: insufficient operating space");
        }
        let mut data = self.rw.read().await.expanders.file_pool.clone();
        let mut roots: Vec<Vec<u8>> = vec![vec![]; num as usize];
        let mut aux = vec![0u8; DEFAULT_AUX_SIZE as usize * tree::DEFAULT_HASH_SIZE as usize];
        let mut mht = tree::get_light_mht(DEFAULT_AUX_SIZE);
        for i in 1..num {
            let cluster = (self.rw.read().await.front + i - 1) / self.rw.read().await.cluster_size + 1;
            let subfile = (self.rw.read().await.front + i - 1) % self.rw.read().await.cluster_size;
            self.read_file_labels(cluster, subfile, &mut data)
                .await
                .context("prove deletion error")?;
            self.read_aux_data(cluster, subfile, &mut aux)
                .await
                .context("prove deletion error")?;
            tree::calc_light_mht_with_aux(&mut mht, &aux);
            roots[i as usize - 1] = tree::get_root(&mht);
        }
        let (wit_chain, acc_path) = self
            .rw
            .write()
            .await
            .acc_manager
            .as_mut()
            .unwrap()
            .delete_elements_and_proof(num)
            .await
            .context("prove deletion error")?;

        let proof = DeletionProof { roots, wit_chain, acc_path };

        Ok(proof)
    }

    pub async fn organize_files(&mut self, idx: i64, num: i64) -> Result<()> {
        let cluster_size = self.rw.read().await.cluster_size;
        let set_len = self.rw.read().await.set_len;
        let k = self.rw.read().await.expanders.k;

        let dir = Path::new(IDLE_FILE_PATH).join(format!("{}-{}", SET_DIR_NAME, idx / (cluster_size * set_len) + 1));

        let mut i = idx + 1;
        while i <= idx + num {
            for j in 0..k {
                // delete idle file
                let name = dir
                    .join(format!("{}-{}", CLUSTER_DIR_NAME, (i - 1) / cluster_size + 1))
                    .join(format!("{}-{}", FILE_NAME, j));
                util::delete_file(name.to_str().unwrap())?;

                // delete aux file
                let name = dir
                    .join(format!("{}-{}", CLUSTER_DIR_NAME, (i - 1) / cluster_size + 1))
                    .join(format!("{}-{}", AUX_FILE, j));
                util::delete_file(name.to_str().unwrap())?;
            }
            i += 8;
        }
        let name = dir.join(COMMIT_FILE);
        util::delete_file(name.to_str().unwrap())?;

        self.rw.write().await.space += num / cluster_size * k * FILE_SIZE;

        Ok(())
    }

    pub async fn delete_files(&mut self) -> Result<()> {
        // // delete all files before front
        let prove_guard = self.rw.read().await;
        let mut indexs: Vec<i64> = vec![];
        indexs.push((prove_guard.front - 1) / (prove_guard.set_len * prove_guard.cluster_size) + 1); //idle-files-i
        indexs.push((prove_guard.front - 1) / prove_guard.cluster_size + 1); //file-cluster-i
        indexs.push((prove_guard.front - 1) % prove_guard.cluster_size + prove_guard.expanders.k); //sub-file-i

        deleter(IDLE_FILE_PATH, indexs).context("delete idle files error")
    }

    pub async fn calc_generated_file(&mut self, dir: &str) -> Result<i64> {
        let mut count = 0_i64;
        let prover_guard = self.rw.read().await;
        let file_total_size =
            prover_guard.config.file_size * (prover_guard.expanders.k + prover_guard.cluster_size) * 1024 * 1024;
        let root_size = (prover_guard.set_len * (prover_guard.expanders.k + prover_guard.cluster_size) + 1)
            * (DEFAULT_HASH_SIZE as i64);
        let mut next = 1_i64;

        let mut files = fs::read_dir(dir).await?;
        while let Some(file) = files.next_entry().await? {
            let file_name = file
                .file_name()
                .into_string()
                .map_err(|_| anyhow!("failed to convert file name to string"))?;
            let sidxs = file_name.split("-").collect::<Vec<&str>>();
            if sidxs.len() < 3 {
                continue;
            }
            let number: i64 = sidxs[2].parse()?;
            if number != prover_guard.rear / (prover_guard.set_len * prover_guard.cluster_size) + next {
                continue;
            }
            if !file.file_type().await?.is_dir() {
                continue;
            }
            let roots_file = file.path().join(COMMIT_FILE);
            match fs::metadata(roots_file).await {
                Ok(metadata) => {
                    if metadata.len() != root_size as u64 {
                        continue;
                    }
                },
                Err(_) => continue,
            }

            let mut clusters = fs::read_dir(file.path()).await?;
            let mut i = 0;
            while let Some(cluster) = clusters.next_entry().await? {
                if !cluster.metadata().await?.is_dir() {
                    continue;
                }

                let mut size = 0;
                let mut files = fs::read_dir(cluster.path()).await?;

                while let Some(file) = files.next_entry().await? {
                    if !file.metadata().await?.is_dir() && file.metadata().await?.len() >= MINIFILESIZE as u64 {
                        size += file.metadata().await?.len() as i64;
                    }
                }
                if size == file_total_size {
                    count += prover_guard.cluster_size;
                    i += 1;
                }
            }
            if i == prover_guard.set_len as usize {
                next += 1;
            }
        }
        Ok(count)
    }
}

pub fn deleter(root_dir: &str, indexs: Vec<i64>) -> Result<()> {
    if indexs.is_empty() {
        return Ok(());
    }

    let entries = std::fs::read_dir(root_dir)?;

    for entry in entries {
        let entry = entry?;
        let file_name = entry.file_name();
        let names: Vec<&str> = file_name.to_str().unwrap().split('-').collect();

        let idx: i64 = match names[names.len() - 1].parse() {
            Ok(idx) => idx,
            Err(_) => continue,
        };

        if idx < indexs[0] || (idx == indexs[0] && !entry.path().is_dir()) {
            util::delete_file(Path::new(root_dir).join(entry.path().to_str().unwrap()).to_str().unwrap())?;
            continue;
        }

        if idx != indexs[0] {
            continue;
        }

        // Recursive call
        if let Err(e) = deleter(entry.path().to_str().unwrap(), indexs[1..].to_vec()) {
            return Err(e);
        }
    }

    Ok(())
}
