use std::sync::Arc;

use tokio::{fs, sync::RwLock};

use crate::acc::file_manager::{recovery_acc_data, save_acc_data};

use super::{file_manager::*, gen_wits_for_acc_nodes, generate_acc, generate_witness, hash_2_prime::h_prime, RsaKey};
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use num_bigint_dig::BigUint;
use rand::Rng;
use serde::{Deserialize, Serialize};

pub const DEFAULT_PATH: &str = "./acc/";
pub const DEFAULT_ELEMS_NUM: i32 = 256;
pub const DEFAULT_LEVEL: i32 = 3;
pub const DEFAULT_NAME: &str = "sub-acc";
pub const DEFAULT_BACKUP_NAME: &str = "sub-acc";

#[async_trait]
pub trait AccHandle {
    async fn get_snapshot(&mut self) -> Arc<RwLock<MutiLevelAcc>>;

    async fn add_elements_and_proof(&mut self, elems: Vec<Vec<u8>>) -> Result<(WitnessNode, Vec<Vec<u8>>)>;

    async fn delete_elements_and_proof(&mut self, num: i64) -> Result<(WitnessNode, Vec<Vec<u8>>)>;

    async fn get_witness_chains(&mut self, indexes: Vec<i64>) -> Result<Vec<WitnessNode>>;

    async fn update_snapshot(&mut self) -> bool;

    async fn rollback(&mut self) -> bool;

    async fn restore_sub_acc_file(&self, index: i64, elems: Vec<Vec<u8>>) -> Result<()>;

    async fn get_file_path(&self) -> &str;
}

#[derive(Clone, Debug, Default)]
pub struct AccNode {
    pub value: Vec<u8>,
    pub children: Vec<Arc<RwLock<AccNode>>>,
    pub len: i64,
    pub wit: Vec<u8>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AccData {
    pub values: Vec<Vec<u8>>,
    pub wits: Vec<Vec<u8>>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct WitnessNode {
    pub elem: Vec<u8>,
    pub wit: Vec<u8>,
    pub acc: Option<Box<WitnessNode>>,
}

#[derive(Clone, Debug)]
pub struct MutiLevelAcc {
    pub accs: Option<Arc<RwLock<AccNode>>>,
    pub key: RsaKey,
    pub elem_nums: i64,
    pub deleted: i64,
    pub curr_count: i64,
    pub curr: Option<Arc<RwLock<AccNode>>>,
    pub parent: Option<Arc<RwLock<AccNode>>>,
    pub snapshot: Option<Arc<RwLock<MutiLevelAcc>>>,
    pub file_path: String,
}

impl Default for MutiLevelAcc {
    fn default() -> Self {
        Self {
            accs: Default::default(),
            key: Default::default(),
            elem_nums: 0,
            deleted: 0,
            curr_count: 0,
            curr: Default::default(),
            parent: Default::default(),
            snapshot: None,
            file_path: "".to_string(),
        }
    }
}

pub async fn recovery(acc_path: &str, key: RsaKey, front: i64, rear: i64) -> Result<MutiLevelAcc> {
    let acc_path = if acc_path.is_empty() { DEFAULT_PATH } else { acc_path };

    if !std::path::Path::new(&acc_path).exists() {
        fs::create_dir_all(&acc_path)
            .await
            .context("recovery muti-acc error:Failed to create directory for the accumulator")?;
    }
    let acc = AccNode { value: key.g.to_bytes_be(), ..Default::default() };
    let mut acc_manager = MutiLevelAcc {
        accs: Some(Arc::new(RwLock::new(acc))),
        key,
        file_path: acc_path.to_string(),
        deleted: front,
        elem_nums: 0,
        curr_count: 0,
        curr: None,
        parent: None,
        snapshot: None,
    };
    acc_manager.construct_muti_acc(rear).await.context("recovery muti-acc error")?;
    Ok(acc_manager)
}

pub async fn new_muti_level_acc(path: &str, key: RsaKey) -> Result<MutiLevelAcc> {
    let path = if path == "" { DEFAULT_PATH.to_string() } else { path.to_string() };

    if !std::path::Path::new(&path).exists() {
        fs::create_dir_all(&path).await?;
    }
    let mut acc = AccNode::default();
    acc.value = key.g.to_bytes_be();

    let mut acc_manager = MutiLevelAcc::default();

    acc_manager.accs = Some(Arc::new(RwLock::new(acc)));
    acc_manager.key = key;
    acc_manager.file_path = path;

    Ok(acc_manager)
}

#[async_trait]
impl AccHandle for MutiLevelAcc {
    async fn get_snapshot(&mut self) -> Arc<RwLock<MutiLevelAcc>> {
        if self.snapshot.is_none() {
            self.create_snap_shot().await;
        };
        self.snapshot.clone().unwrap()
    }

    // AddElementsAndProof adds elements to muti-level acc and create proof of added elements
    async fn add_elements_and_proof(&mut self, elems: Vec<Vec<u8>>) -> Result<(WitnessNode, Vec<Vec<u8>>)> {
        let snapshot = self.get_snapshot().await;
        let mut exist = WitnessNode {
            elem: snapshot.read().await.accs.clone().unwrap().read().await.value.clone(),
            ..Default::default()
        };

        self.add_elements(elems).await.context("prove acc insertion proof")?;
        //the proof of adding elements consists of two parts,
        //the first part is the witness chain of the bottom accumulator where the element is located,
        //witness chain node is a special structure(Elem(acc value) is G,Wit is parent node's Elem)
        //when inserting an element needs to trigger the generation of a new accumulator,
        //the second part is an accumulator list, which contains the accumulator value
        //recalculated from the bottom to the top after inserting elements
        let mut count = 1;
        let mut p = self.accs.clone();
        let mut q = self.snapshot.clone().unwrap().read().await.accs.clone();
        while p.is_some() && q.is_some() && count < DEFAULT_LEVEL as usize {
            if p.clone().unwrap().read().await.len > q.clone().unwrap().read().await.len {
                for _ in count..DEFAULT_LEVEL as usize {
                    exist = WitnessNode { acc: Some(Box::new(exist)), ..Default::default() };
                    exist.elem = self.key.g.to_bytes_be();
                    exist.wit = exist.acc.as_ref().unwrap().elem.clone();
                }
                break;
            }
            count += 1;
            let p_len = p.clone().unwrap().read().await.len as usize;
            let q_len = q.clone().unwrap().read().await.len as usize;
            p = Some(p.clone().unwrap().read().await.children[p_len - 1].clone());
            q = Some(q.clone().unwrap().read().await.children[q_len - 1].clone());

            exist = WitnessNode { acc: Some(Box::new(exist)), ..Default::default() };
            exist.elem = q.clone().unwrap().read().await.value.clone();
            exist.wit = q.clone().unwrap().read().await.wit.clone();
        }

        p = self.accs.clone();
        let mut accs = vec![vec![]; DEFAULT_LEVEL as usize];
        for i in 0..DEFAULT_LEVEL {
            accs[(DEFAULT_LEVEL - i - 1) as usize] = p.clone().unwrap().read().await.value.clone();
            if p.clone().unwrap().read().await.children.len() > 0 {
                let p_len = p.clone().unwrap().read().await.len as usize;
                p = Some(p.clone().clone().unwrap().read().await.children[p_len - 1].clone());
            };
        }

        Ok((exist, accs))
    }

    // DeleteElementsAndProof deletes elements from muti-level acc and create proof of deleted elements
    async fn delete_elements_and_proof(&mut self, num: i64) -> Result<(WitnessNode, Vec<Vec<u8>>)> {
        if self.elem_nums == 0 {
            bail!("delete null set:prove acc deletion proof error")
        }

        //Before deleting elements, get their chain of witness
        let exist = WitnessNode {
            elem: self.accs.clone().unwrap().read().await.children[0].read().await.children[0]
                .read()
                .await
                .value
                .clone(),
            wit: self.accs.clone().unwrap().read().await.children[0].read().await.children[0]
                .read()
                .await
                .wit
                .clone(),
            acc: Some(Box::new(WitnessNode {
                elem: self.accs.clone().unwrap().read().await.children[0].read().await.value.clone(),
                wit: self.accs.clone().unwrap().read().await.children[0].read().await.wit.clone(),
                acc: Some(Box::new(WitnessNode {
                    elem: self.accs.clone().unwrap().read().await.value.clone(),
                    wit: Vec::new(),
                    acc: None,
                })),
            })),
        };

        let snapshot = self.get_snapshot().await;

        self.delete_elements(num).await.context("prove acc deletion proof error")?;

        let mut accs = vec![vec![]; DEFAULT_LEVEL as usize];
        accs[DEFAULT_LEVEL as usize - 1] = self.accs.clone().unwrap().read().await.value.clone();
        let mut count = 1;
        let mut p = self.accs.clone();
        let mut q = snapshot.read().await.accs.clone();
        while p.is_some() && q.is_some() && count < DEFAULT_LEVEL as usize {
            if p.clone().unwrap().read().await.len < q.clone().unwrap().read().await.len {
                for i in (DEFAULT_LEVEL as usize - count - 1)..=0 {
                    accs[i] = self.key.g.to_bytes_be();
                }
                break;
            }
            count += 1;
            p = Some(p.clone().unwrap().read().await.children[0].clone());
            q = Some(q.clone().unwrap().read().await.children[0].clone());
            accs[DEFAULT_LEVEL as usize - count] = p.clone().unwrap().read().await.value.clone();
        }

        Ok((exist, accs))
    }

    // get witness chains for prove space challenge
    async fn get_witness_chains(&mut self, indexes: Vec<i64>) -> Result<Vec<WitnessNode>> {
        let mut data = AccData::default();
        let snapshot = self.get_snapshot().await;
        let mut chains = Vec::new();
        let mut fidx = -1;

        for i in 0..indexes.len() {
            if indexes[i] <= self.deleted || indexes[i] > self.deleted + self.elem_nums {
                bail!("bad index")
            }

            if (indexes[i] - 1) / DEFAULT_ELEMS_NUM as i64 > fidx {
                fidx = (indexes[i] - 1) / DEFAULT_ELEMS_NUM as i64;
                data = read_acc_data(&self.file_path, fidx as i64)?;
            }

            let chain = snapshot
                .clone()
                .write()
                .await
                .get_witness_chain(indexes[i], &data)
                .await
                .context("get witness chains error")?;
            chains.push(chain);
        }
        Ok(chains)
    }

    //UpdateSnapshot will update acc's snapshot,this method should be called after acc updated
    async fn update_snapshot(&mut self) -> bool {
        self.create_snap_shot().await;
        true
    }

    //RollBack will roll back acc to snapshot version,please use with caution
    async fn rollback(&mut self) -> bool {
        if self.snapshot.is_none() {
            return false;
        }
        if self.deleted != self.snapshot.clone().unwrap().read().await.deleted {
            if recovery_acc_data(&self.file_path, self.deleted as i64 / DEFAULT_ELEMS_NUM as i64).is_err() {
                return false;
            };
        };
        let other = self.snapshot.clone().unwrap();
        let other_guard = other.read().await;
        let mut accs = AccNode::default();
        copy_acc_node(&other_guard.accs.clone().unwrap().read().await.clone(), &mut accs).await;
        self.accs = Some(Arc::new(RwLock::new(accs)));
        self.key = other_guard.key.clone();
        self.elem_nums = other_guard.elem_nums;
        self.curr_count = other_guard.curr_count;

        if self.accs.clone().unwrap().read().await.len > 0 {
            let index = self.accs.clone().unwrap().read().await.len as usize - 1;
            self.parent = Some(self.accs.clone().unwrap().read().await.children[index].clone());
        }

        if let Some(ref parent) = self.parent {
            if parent.read().await.len > 0 {
                let index = self.parent.clone().unwrap().read().await.len as usize - 1;
                self.curr = Some(self.parent.clone().unwrap().read().await.children[index].clone());
            }
        }
        self.deleted = other_guard.deleted;
        self.file_path = other_guard.file_path.clone();
        true
    }

    async fn restore_sub_acc_file(&self, index: i64, elems: Vec<Vec<u8>>) -> Result<()> {
        if elems.len() != DEFAULT_ELEMS_NUM as usize {
            bail!("wrong number of elements")
        }
        let mut data = AccData { values: elems, wits: Vec::new() };
        data.wits = generate_witness(self.key.g.clone(), self.key.n.clone(), data.values.clone()).await;

        save_acc_data(self.file_path.as_str(), index, data.values, data.wits)
    }

    async fn get_file_path(&self) -> &str {
        &self.file_path
    }
}

impl MutiLevelAcc {
    pub async fn create_snap_shot(&mut self) {
        // self.snapshot = Some(Arc::new(RwLock::new(MutiLevelAcc::default())));
        let mut new_snapshot = MutiLevelAcc::default();
        let mut accs = AccNode::default();
        copy_acc_node(&self.accs.clone().unwrap().read().await.clone(), &mut accs).await;
        new_snapshot.accs = Some(Arc::new(RwLock::new(accs)));
        new_snapshot.key = self.key.clone();
        new_snapshot.elem_nums = self.elem_nums;
        new_snapshot.curr_count = self.curr_count;

        if new_snapshot.accs.clone().unwrap().read().await.len > 0 {
            let index = new_snapshot.accs.clone().unwrap().read().await.len as usize - 1;
            new_snapshot.parent = Some(new_snapshot.accs.clone().unwrap().read().await.children[index].clone());
        }
        if let Some(ref parent) = new_snapshot.parent {
            if parent.read().await.len > 0 {
                let index = new_snapshot.parent.clone().unwrap().read().await.len as usize - 1;
                new_snapshot.curr = Some(new_snapshot.parent.clone().unwrap().read().await.children[index].clone());
            }
        }
        new_snapshot.deleted = self.deleted;
        new_snapshot.file_path = self.file_path.clone();
        self.snapshot = Some(Arc::new(RwLock::new(new_snapshot)));
    }

    // pub async fn set_update(&mut self, yes: bool) {}

    pub async fn add_elements(&mut self, elems: Vec<Vec<u8>>) -> Result<()> {
        let lens = elems.len();
        // the range of length of elems to insert is [0, 1024]
        if lens == 0
            || (self.curr_count < DEFAULT_ELEMS_NUM as i64 && lens as i64 + self.curr_count > DEFAULT_ELEMS_NUM as i64)
        {
            bail!("add elements error:illegal number of elements")
        }
        let new_acc = self.add_elements_internal(elems).await.context("add elements error")?;
        self.add_sub_acc(new_acc).await;
        Ok(())
    }

    async fn add_elements_internal(&mut self, elems: Vec<Vec<u8>>) -> Result<AccNode> {
        let mut node = AccNode::default();

        let mut data = if self.curr_count > 0 && self.curr_count < DEFAULT_ELEMS_NUM as i64 {
            let index = (self.deleted + self.elem_nums - 1) / DEFAULT_ELEMS_NUM as i64;
            let mut data = read_acc_data(&self.file_path, index).context("add elements to sub acc error")?;
            data.values.extend_from_slice(&elems);
            data
        } else {
            let mut data = AccData::default();
            data.values = elems.clone();
            data
        };

        data.wits = generate_witness(self.key.g.clone(), self.key.n.clone(), data.values.clone()).await;
        node.len = data.values.len() as i64;
        node.value = generate_acc(
            &self.key,
            &data.wits[(node.len - 1) as usize],
            vec![data.values[(node.len - 1) as usize].clone()],
        )
        .unwrap();

        let index = ((self.deleted + self.elem_nums + elems.len() as i64) - 1) / DEFAULT_ELEMS_NUM as i64;

        save_acc_data(&self.file_path, index, data.values, data.wits).context("add elements to sub acc error")?;

        Ok(node)
    }

    // addSubAccs inserts the sub acc built with new elements into the multilevel accumulator
    pub async fn add_sub_acc(&mut self, mut sub_acc: AccNode) {
        // acc.CurrCount will be equal to zero when the accumulator is empty
        if self.curr_count == 0 {
            sub_acc.wit = self.key.g.to_bytes_be();
            self.curr_count = sub_acc.len;
            self.curr = Some(Arc::new(RwLock::new(sub_acc)));
            self.parent = Some(Arc::new(RwLock::new(AccNode {
                value: generate_acc(
                    &self.key,
                    &self.key.g.to_bytes_be(),
                    vec![self.curr.clone().unwrap().read().await.value.clone()],
                )
                .unwrap(),
                wit: self.key.g.to_bytes_be(),
                children: vec![self.curr.clone().unwrap()],
                len: 1,
            })));
            self.accs = Some(Arc::new(RwLock::new(AccNode {
                value: generate_acc(
                    &self.key,
                    &self.key.g.to_bytes_be(),
                    vec![self.parent.clone().unwrap().read().await.value.clone()],
                )
                .unwrap(),
                children: vec![self.parent.clone().unwrap()],
                len: 1,
                wit: Vec::new(),
            })));
            self.elem_nums += self.curr_count;
            return;
        }

        let sub_acc_pointer = Arc::new(RwLock::new(sub_acc.clone()));
        // The upper function has judged that acc.CurrCount + elemNums is less than or equal DEFAULT_ELEMS_NUM
        if self.curr_count > 0 && self.curr_count < DEFAULT_ELEMS_NUM as i64 {
            self.elem_nums += sub_acc.len - self.curr_count;
            let lens = self.parent.clone().unwrap().read().await.children.len();
            self.parent.clone().unwrap().write().await.children[lens - 1] = sub_acc_pointer.clone();
        } else if self.parent.clone().unwrap().read().await.children.len() + 1 <= DEFAULT_ELEMS_NUM as usize {
            self.elem_nums += sub_acc.len;
            self.parent
                .clone()
                .unwrap()
                .write()
                .await
                .children
                .push(sub_acc_pointer.clone());
        } else {
            self.elem_nums += sub_acc.len;
            let node = Arc::new(RwLock::new(AccNode {
                value: Vec::new(),
                children: vec![sub_acc_pointer.clone()],
                len: 1,
                wit: self.key.g.to_bytes_be(),
            }));
            self.accs.clone().unwrap().write().await.children.push(node.clone());
            self.parent = Some(node.clone())
        }

        self.curr = Some(sub_acc_pointer.clone());
        self.curr_count = self.curr.clone().unwrap().read().await.len as i64;
        // Update sibling witness and parent acc
        self.parent.clone().unwrap().write().await.update_acc(&self.key).await;
        // Update parents and top acc
        self.accs.clone().unwrap().write().await.update_acc(&self.key).await;
    }

    // addSubAccBybatch inserts the sub acc built with new elements into the multilevel accumulator,
    // However, the lazy update mechanism is adopted, and the final update is performed after the accumulator is built.
    pub async fn add_sub_acc_by_batch(&mut self, sub_acc: AccNode) {
        let sub_acc_pointer = Arc::new(RwLock::new(sub_acc.clone()));
        // acc.CurrCount will be equal to zero when the accumulator is empty
        if self.curr_count == 0 {
            self.curr = Some(sub_acc_pointer.clone());
            self.curr_count = self.curr.as_ref().unwrap().read().await.len;
            let parent_node = AccNode { children: vec![self.curr.clone().unwrap()], len: 1, ..Default::default() };
            self.parent = Some(Arc::new(RwLock::new(parent_node)));

            let acc_node = AccNode { children: vec![self.parent.clone().unwrap()], len: 1, ..Default::default() };
            self.accs = Some(Arc::new(RwLock::new(acc_node)));
            self.elem_nums += self.curr_count;
            return;
        }

        // The upper function has judged that acc.CurrCount + elemNums is less than or equal DEFAULT_ELEMS_NUM
        if self.curr_count > 0 && self.curr_count < DEFAULT_ELEMS_NUM as i64 {
            self.elem_nums += sub_acc.len - self.curr_count;
            let lens = self.parent.clone().unwrap().read().await.children.len();
            self.parent.clone().unwrap().write().await.children[lens - 1] = sub_acc_pointer.clone();
        } else if self.parent.clone().unwrap().read().await.children.len() + 1 <= DEFAULT_ELEMS_NUM as usize {
            self.elem_nums += sub_acc.len;
            self.parent
                .clone()
                .unwrap()
                .write()
                .await
                .children
                .push(sub_acc_pointer.clone());
        } else {
            self.elem_nums += sub_acc.len;
            let node = AccNode { children: vec![sub_acc_pointer.clone()], len: 1, ..Default::default() };
            let node_pointer = Arc::new(RwLock::new(node));
            self.accs.clone().unwrap().write().await.children.push(node_pointer.clone());
            self.parent = Some(node_pointer);
        }

        self.curr = Some(sub_acc_pointer.clone());
        self.curr_count = self.curr.clone().unwrap().read().await.len;
    }

    pub async fn delete_elements(&mut self, num: i64) -> Result<()> {
        let index = self.deleted / DEFAULT_ELEMS_NUM as i64;
        let offset = self.deleted % DEFAULT_ELEMS_NUM as i64;

        if num <= 0 || num > self.elem_nums || num + offset > DEFAULT_ELEMS_NUM as i64 {
            bail!("illegal number of elements")
        }

        // Read data from disk
        let mut data = read_acc_data(&self.file_path, index).context("delet elements error")?;
        // Backup file
        backup_acc_data(&self.file_path, index).context("delet elements error")?;

        // Delete elements from acc and update acc
        if num < data.values.len() as i64 {
            data.values = data.values[num as usize..].to_vec();
            data.wits = generate_witness(self.key.g.clone(), self.key.n.clone(), data.values.clone()).await;
            save_acc_data(&self.file_path, index, data.values.clone(), data.wits.clone())?;
            self.accs.clone().unwrap().write().await.children[0].write().await.children[0]
                .write()
                .await
                .len -= num;
            let len = self.accs.clone().unwrap().read().await.children[0].read().await.children[0]
                .read()
                .await
                .len;
            self.accs.clone().unwrap().write().await.children[0].write().await.children[0]
                .write()
                .await
                .value =
                generate_acc(&self.key, &data.wits[(len - 1) as usize], vec![data.values[(len - 1) as usize].clone()])
                    .unwrap();
        } else {
            delete_acc_data(&self.file_path, index as i32).context("delet elements error")?;

            // Update mid-level acc
            self.accs.clone().unwrap().write().await.children[0].write().await.children =
                self.accs.clone().unwrap().read().await.children[0].read().await.children[1..].to_vec();
            self.accs.clone().unwrap().write().await.children[0].write().await.len -= 1;
            if self.accs.clone().unwrap().read().await.children[0].read().await.len == 0
                && self.accs.as_ref().unwrap().read().await.len >= 1
            {
                self.accs.clone().unwrap().write().await.children =
                    self.accs.clone().unwrap().read().await.children[1..].to_vec();
                self.accs.clone().unwrap().write().await.len -= 1;
            }

            // Update top-level acc
            if self.accs.clone().unwrap().read().await.len == 0 {
                self.parent = None;
                self.curr = None;
                self.curr_count = 0;
            }
        }

        self.elem_nums -= num;
        // Update sibling witness and parent acc
        self.accs.clone().unwrap().write().await.children[0]
            .clone()
            .write()
            .await
            .update_acc(&self.key)
            .await;
        // Update parents and top acc
        self.accs.clone().unwrap().write().await.update_acc(&self.key).await;
        self.deleted += num;
        Ok(())
    }

    pub async fn get_witness_chain(&mut self, index: i64, data: &AccData) -> Result<WitnessNode> {
        let idx = (index - (DEFAULT_ELEMS_NUM as i64 - data.values.len() as i64) - 1) % DEFAULT_ELEMS_NUM as i64;
        let index = index - (self.deleted - self.deleted % DEFAULT_ELEMS_NUM as i64);
        let mut p = self.accs.clone().unwrap();
        let mut wit = WitnessNode::default();
        let mut i = 0;

        for _ in 0..DEFAULT_LEVEL {
            if i == 0 {
                wit = WitnessNode { elem: p.read().await.value.clone(), wit: p.read().await.wit.clone(), acc: None };
            } else {
                wit = WitnessNode {
                    elem: p.read().await.value.clone(),
                    wit: p.read().await.wit.clone(),
                    acc: Some(Box::new(wit)),
                };
            }

            let size = (DEFAULT_ELEMS_NUM as f64).powf((DEFAULT_LEVEL - i - 1) as f64) as i64;
            let mut idx = (index - 1) / size;
            idx = idx % size;

            if p.read().await.children.len() < idx as usize + 1 || p.read().await.children.is_empty() {
                i += 1;
                continue;
            }
            let p_child = p.read().await.children[idx as usize].clone();

            p = p_child.clone();
            i += 1;
        }

        if i < DEFAULT_LEVEL {
            return Err(anyhow::anyhow!("get witness node error").into());
        }

        let wit_node = WitnessNode {
            elem: data.values[idx as usize].clone(),
            wit: data.wits[idx as usize].clone(),
            acc: Some(Box::new(wit)),
        };

        Ok(wit_node)
    }

    pub async fn construct_muti_acc(&mut self, rear: i64) -> Result<()> {
        if rear == self.deleted {
            return Ok(());
        }
        let num = (rear - self.deleted - 1) / DEFAULT_ELEMS_NUM as i64;
        let offset = self.deleted % DEFAULT_ELEMS_NUM as i64;
        for i in 0..=num {
            let index = self.deleted / DEFAULT_ELEMS_NUM as i64 + i;
            let mut backup = read_backup(&self.file_path, index)?;
            if backup.values.len() + offset as usize != DEFAULT_ELEMS_NUM as usize {
                backup = read_acc_data(&self.file_path, index)?;
            } else {
                recovery_acc_data(&self.file_path, index)?;
            }

            let mut node = AccNode::default();
            let right = backup.values.len();
            if i == 0 && (DEFAULT_ELEMS_NUM as i64 - offset as i64) < (right as i64) {
                let left = self.deleted % DEFAULT_ELEMS_NUM as i64 - (DEFAULT_ELEMS_NUM as i64 - right as i64); //sub real file offset
                backup.values = backup.values[left as usize..right].to_vec();
                backup.wits = generate_witness(self.key.g.clone(), self.key.n.clone(), backup.values.clone()).await;

                save_acc_data(&self.file_path, index, backup.values.clone(), backup.wits.clone())?;
            }

            node.len = backup.values.len() as i64;
            node.value = generate_acc(
                &self.key,
                &backup.wits[(node.len - 1) as usize],
                vec![backup.values[(node.len - 1) as usize].clone()],
            )
            .unwrap();
            self.add_sub_acc_by_batch(node).await;
            if i == 0 && offset > 0 {
                self.curr_count += self.deleted % DEFAULT_ELEMS_NUM as i64;
            }
        }

        // Update the upper accumulator and its evidence
        for acc in self.accs.clone().unwrap().write().await.children.iter_mut() {
            acc.write().await.update_acc(&self.key).await;
        }
        self.accs.clone().unwrap().write().await.update_acc(&self.key).await;

        Ok(())
    }
}

impl AccNode {
    pub async fn update_acc(&mut self, key: &RsaKey) {
        let lens = self.children.len();
        self.len = lens as i64;
        if lens == 0 {
            self.value = key.g.to_bytes_be();
            self.wit = Vec::new();
            return;
        }
        gen_wits_for_acc_nodes(&key.g, &key.n, &mut self.children).await;
        let last = self.children[lens - 1].clone();
        self.value = generate_acc(key, &last.read().await.wit, vec![last.read().await.value.clone()]).unwrap();
    }
}

async fn copy_acc_node(src: &AccNode, target: &mut AccNode) {
    target.value = src.value.clone();
    target.children = src.children.clone();
    target.len = src.len;
    target.wit = src.wit.clone();
    for child in &src.children {
        let child_guard = child.read().await.clone();
        let mut new_child = AccNode {
            value: child_guard.value.clone(),
            children: child_guard.children.clone(),
            len: child_guard.len,
            wit: child_guard.wit.clone(),
        };
        Box::pin(copy_acc_node(&child_guard, &mut new_child)).await;
        target.children.push(Arc::new(RwLock::new(new_child)));
    }
}

pub fn verify_insert_update(
    key: RsaKey,
    exist: Option<Box<WitnessNode>>,
    elems: Vec<Vec<u8>>,
    accs: Vec<Vec<u8>>,
    acc: Vec<u8>,
) -> bool {
    if exist.is_none() || elems.is_empty() || accs.len() < DEFAULT_LEVEL as usize {
        println!("acc wit chain is empty");
        return false;
    }

    let mut p = exist.clone().unwrap().as_ref().clone();
    while p.acc.is_some() && p.acc.as_ref().unwrap().elem == p.wit {
        p = p.acc.unwrap().as_ref().clone();
    }

    // Proof of the witness of accumulator elements,
    // when the element's accumulator does not exist, recursively verify its parent accumulator
    if !verify_mutilevel_acc(&key, Some(&mut p.clone()), &acc) {
        println!("verify muti-level acc error");
        return false;
    }

    // Verify that the newly generated accumulators after inserting elements
    // is calculated based on the original accumulators
    let sub_acc = generate_acc(&key, &exist.as_ref().unwrap().elem, elems);
    if !sub_acc.eq(&Some(accs[0].clone())) {
        println!("Verify that the newly generated accumulators after inserting elements is calculated based on the original accumulators error");
        return false;
    }

    let mut count = 1;
    let mut p = *exist.unwrap();
    let mut sub_acc;

    while p.acc.is_some() {
        sub_acc = generate_acc(&key, &p.wit, vec![accs[count - 1].clone()]);

        if !sub_acc.eq(&Some(accs[count].to_vec())) {
            println!("verify sub acc error");
            return false;
        }
        p = *p.acc.unwrap();
        count += 1;
    }

    true
}

fn verify_acc(key: &RsaKey, acc: &[u8], u: &[u8], wit: &[u8]) -> bool {
    let e = h_prime(&BigUint::from_bytes_be(u));
    let dash = BigUint::from_bytes_be(wit).modpow(&e, &key.n);
    dash == BigUint::from_bytes_be(acc)
}

pub fn verify_mutilevel_acc(key: &RsaKey, wits: Option<&mut WitnessNode>, acc: &[u8]) -> bool {
    let mut current_wit = wits.unwrap();
    while let Some(acc_node) = &mut current_wit.acc {
        if !verify_acc(key, &acc_node.elem, &current_wit.elem, &current_wit.wit) {
            return false;
        }
        current_wit = acc_node;
    }
    current_wit.elem.eq(acc)
}

pub fn verify_mutilevel_acc_for_batch(key: &RsaKey, base_idx: i64, wits: Vec<WitnessNode>, acc: &[u8]) -> bool {
    let mut sub_acc: Option<Vec<u8>> = None;
    let default_elems_num = DEFAULT_ELEMS_NUM as i64;
    for (i, witness) in wits.iter().enumerate() {
        if let Some(sa) = &sub_acc {
            if witness.acc.clone().unwrap().elem != *sa {
                return false;
            }
        }

        if (i as i64 + base_idx) % default_elems_num == 0 || i == wits.len() - 1 {
            if !verify_mutilevel_acc(key, Some(&mut witness.clone()), acc) {
                return false;
            }
            sub_acc = None;
            continue;
        }

        let mut rng = rand::thread_rng();
        if rng.gen_range(0..100) < 25
            && !verify_acc(key, &witness.acc.clone().unwrap().elem, &witness.elem, &witness.wit)
        {
            return false;
        }

        sub_acc = Some(witness.acc.clone().unwrap().elem.clone());
    }
    true
}

pub fn verify_delete_update(
    key: RsaKey,
    exist: &mut WitnessNode,
    elems: Vec<Vec<u8>>,
    accs: Vec<Vec<u8>>,
    acc: &[u8],
) -> bool {
    if elems.is_empty() || accs.len() < DEFAULT_LEVEL as usize {
        return false;
    }
    if !verify_mutilevel_acc(&key, Some(exist), acc) {
        return false;
    }

    let mut sub_acc = generate_acc(&key, &accs[0], elems);
    if sub_acc.eq(&Some(exist.elem.clone())) {
        return false;
    }
    let mut p = exist;
    let mut count = 1;
    while p.acc.is_some() {
        if !accs[count - 1].eq(&key.g.to_bytes_be()) {
            sub_acc = generate_acc(&key, &p.wit, vec![accs[count - 1].clone()]);
        } else {
            sub_acc = Some(p.wit.clone());
        }
        if !sub_acc.eq(&Some(accs[count].to_vec())) {
            return false;
        }
        p = p.acc.as_mut().unwrap();
        count += 1;
    }

    true
}
