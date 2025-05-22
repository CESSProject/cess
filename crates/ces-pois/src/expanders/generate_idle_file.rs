use std::{
    path::{self, Path},
    vec,
};

use super::{generate_expanders::calc_parents, get_bytes, Expanders, Node, NodeType};
use crate::{
    tree::{self},
    util,
};
use anyhow::{Context, Result};
use sha2::{Digest, Sha256, Sha512};
use tokio::{fs, io::AsyncReadExt};

pub const DEFAULT_IDLE_FILES_PATH: &str = "./proofs";
pub const FILE_NAME: &str = "sub-file";
pub const COMMIT_FILE: &str = "file-roots";
pub const CLUSTER_DIR_NAME: &str = "file-cluster";
pub const SET_DIR_NAME: &str = "idle-files";
pub const AUX_FILE: &str = "aux-file";
pub const DEFAULT_AUX_SIZE: i64 = 64;
pub const DEFAULT_NODES_CACHE: i64 = 1024;

pub const HASH_SIZE: i32 = 64;

pub enum Hasher {
    SHA256(Sha256),
    SHA512(Sha512),
}

pub async fn make_proof_dir(dir: &str) -> Result<()> {
    if fs::metadata(dir).await.is_err() {
        fs::DirBuilder::new().recursive(true).create(dir).await?;
    } else {
        fs::remove_dir_all(dir).await?;
        fs::DirBuilder::new().recursive(true).create(dir).await?;
    }
    Ok(())
}

pub fn new_hash() -> Hasher {
    match HASH_SIZE {
        32 => Hasher::SHA256(Sha256::new()),
        64 => Hasher::SHA512(Sha512::new()),
        _ => Hasher::SHA512(Sha512::new()),
    }
}

pub fn get_hash(data: &[u8]) -> Vec<u8> {
    let hash = new_hash();
    let mut data = data;
    if data.is_empty() {
        data = b"none";
    }

    match hash {
        Hasher::SHA256(hash) => {
            let mut hash = hash;
            hash.update(data);
            let result = hash.finalize();
            result.to_vec()
        },
        Hasher::SHA512(hash) => {
            let mut hash = hash;
            hash.update(data);
            let result = hash.finalize();
            result.to_vec()
        },
    }
}

impl Expanders {
    pub async fn generate_idle_file_set(
        &mut self,
        miner_id: Vec<u8>,
        start: i64,
        size: i64,
        root_dir: &str,
    ) -> Result<()> {
        let mut clusters = vec![0_i64; size as usize];
        let set_dir = format!("{}/{}-{}", root_dir, SET_DIR_NAME, (start + size) / size);

        for i in start..start + size {
            let dir = format!("{}/{}-{}", set_dir, CLUSTER_DIR_NAME, i);
            make_proof_dir(&dir).await.context("generate idle file error")?;
            clusters[(i - start) as usize] = i;
        }

        // Number of idle files in each file cluster
        let file_num = self.k;
        //create aux slices
        let mut roots = vec![vec![0, 0]; (self.k + file_num) as usize * size as usize + 1];
        let mut elders = self.file_pool.clone();
        let mut labels = self.file_pool.clone();
        let mut mht = tree::get_light_mht(self.n);
        let mut aux = vec![0u8; (DEFAULT_AUX_SIZE * tree::DEFAULT_HASH_SIZE as i64) as usize];

        //calc node labels
        let front_size = miner_id.len() + std::mem::size_of::<NodeType>() + 8 + 8;
        let mut label = vec![0u8; front_size + 2 * HASH_SIZE as usize];
        util::copy_data(&mut label, &[&miner_id]);
        // let node = Node::default();

        for i in 0..(self.k + file_num) {
            let mut logical_layer = i;
            for j in 0..size {
                let mut parents = Vec::new();
                util::copy_data(
                    &mut label[miner_id.len()..],
                    &[&get_bytes(clusters[j as usize]), &get_bytes(0 as i64)],
                );
                //calc nodes relationship
                //read parents' label of file j, and fill elder node labels to add files relationship
                if i >= self.k {
                    logical_layer = self.k;
                    //When the last level is reached, join the file index
                    util::copy_data(
                        &mut label[miner_id.len() + 8..],
                        &[&get_bytes((clusters[j as usize] - 1) * file_num + i - self.k + 1)],
                    );
                    self.read_elders_data(&set_dir, i, j, &mut elders, &clusters).await?;
                }

                if i > 0 {
                    fs::File::open(
                        &path::Path::new(&set_dir)
                            .join(format!("{}-{}", CLUSTER_DIR_NAME, clusters[j as usize]))
                            .join(format!("{}-{}", FILE_NAME, logical_layer - 1)),
                    )
                    .await?
                    .read_to_end(&mut parents)
                    .await
                    .context("generate idle file error")?;
                }
                for k in 0..self.n {
                    let mut hasher = Sha512::new();
                    util::copy_data(
                        &mut label[miner_id.len() + 8 + 8..],
                        &[&get_bytes((logical_layer * self.n + k) as NodeType)],
                    );
                    util::clear_data(&mut label[front_size..]);
                    let node = self.calc_nodes_parents(i, &miner_id, clusters[j as usize], k);
                    if i > 0 && !node.no_parents() {
                        for p in node.parents.iter() {
                            let idx = *p as i64 % self.n;
                            let l = idx * HASH_SIZE as i64;
                            let r = (idx + 1) * HASH_SIZE as i64;
                            if (*p as i64) < logical_layer * self.n {
                                util::add_data(
                                    &mut label[front_size..front_size + HASH_SIZE as usize],
                                    &[&parents[l as usize..r as usize]],
                                );
                            } else {
                                // let label_tmp = label.clone()[l as usize..r as usize].to_vec();
                                util::add_data(
                                    &mut label[front_size..front_size + HASH_SIZE as usize],
                                    &[&labels[l as usize..r as usize]],
                                );
                            }
                        }
                        // //add files relationship
                        if i >= self.k {
                            util::add_data(
                                &mut label[front_size + HASH_SIZE as usize..front_size + 2 * HASH_SIZE as usize],
                                &[&elders[(k * HASH_SIZE as i64) as usize..((k + 1) * HASH_SIZE as i64) as usize]],
                            );
                        }
                    }
                    hasher.update(&label);
                    if i + j > 0 {
                        //add same layer dependency relationship
                        hasher.update(&labels[(k * HASH_SIZE as i64) as usize..((k + 1) * HASH_SIZE as i64) as usize]);
                    };
                    labels[(k * HASH_SIZE as i64) as usize..((k + 1) * HASH_SIZE as i64) as usize]
                        .copy_from_slice(&hasher.finalize_reset());
                }

                //calc merkel tree root hash
                tree::calc_light_mht_with_bytes(&mut mht, &labels, HASH_SIZE as i64);
                roots[(i * size + j) as usize] = tree::get_root(&mht);
                aux.copy_from_slice(
                    &mht[DEFAULT_AUX_SIZE as usize * tree::DEFAULT_HASH_SIZE as usize
                        ..2 * DEFAULT_AUX_SIZE as usize * tree::DEFAULT_HASH_SIZE as usize],
                );

                //save aux data
                util::save_file(
                    &Path::new(set_dir.as_str())
                        .join(format!("{}-{}", CLUSTER_DIR_NAME, clusters[j as usize]))
                        .join(format!("{}-{}", AUX_FILE, i)),
                    &aux,
                )?;

                //save one layer labels of one file
                util::save_file(
                    &Path::new(set_dir.as_str())
                        .join(format!("{}-{}", CLUSTER_DIR_NAME, clusters[j as usize]))
                        .join(format!("{}-{}", FILE_NAME, i)),
                    &labels,
                )?;
            }
        }
        //return memory space
        drop(labels);
        drop(elders);
        drop(mht);
        //calculate new dir name
        let mut hasher = Sha256::new();
        for i in 0..roots.len() - 1 {
            hasher.update(&roots[i])
        }
        roots[((self.k + file_num) * size) as usize] = hasher.finalize().to_vec();

        util::save_proof_file(&Path::new(set_dir.as_str()).join(COMMIT_FILE), &roots)?;

        Ok(())
    }

    pub fn calc_nodes_parents(&self, layer: i64, miner_id: &[u8], count: i64, j: i64) -> Node {
        let logical_layer = if layer >= self.k { self.k } else { layer };

        let mut node = self.nodes_pool.clone();
        node.index = (j + logical_layer * self.n) as NodeType;
        node.parents = Vec::with_capacity(self.d as usize + 1);

        calc_parents(self, &mut node, miner_id, count, layer);
        node
    }

    pub async fn read_elders_data(
        &self,
        set_dir: &str,
        layer: i64,
        cidx: i64,
        elders: &mut Vec<u8>,
        clusters: &[i64],
    ) -> Result<()> {
        let base_layer = ((layer - self.k / 2) / self.k) as usize;
        util::clear_data(elders);
        for l in 0..(self.k / 2) as usize {
            let mut temp = Vec::new();
            fs::File::open(
                &path::Path::new(set_dir)
                    .join(format!("{}-{}", CLUSTER_DIR_NAME, clusters[cidx as usize]))
                    .join(format!("{}-{}", FILE_NAME, base_layer + 2 * l)),
            )
            .await?
            .read_to_end(&mut temp)
            .await?;

            util::add_data(elders, &[&temp]);
        }

        Ok(())
    }
}
