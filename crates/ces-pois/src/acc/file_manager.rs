use super::multi_level_acc::{AccData, DEFAULT_BACKUP_NAME, DEFAULT_NAME};
use crate::util;
use anyhow::{Context, Result};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

pub fn save_acc_data(dir: &str, index: i64, elems: Vec<Vec<u8>>, wits: Vec<Vec<u8>>) -> Result<()> {
    let data = AccData { values: elems, wits };

    let jbytes = serde_json::to_vec(&data)?;

    let fpath = Path::new(dir).join(format!("{}-{}", DEFAULT_NAME, index));

    util::save_file(&fpath, &jbytes)
}

pub fn read_acc_data(dir: &str, index: i64) -> Result<AccData> {
    let fpath = format!("{}/{}-{}", dir, DEFAULT_NAME, index);
    read_data(&fpath)
}
pub fn read_backup(dir: &str, index: i64) -> Result<AccData> {
    let fpath = format!("{}/{}-{}", dir, DEFAULT_BACKUP_NAME, index);
    read_data(&fpath)
}

pub fn read_data(fpath: &str) -> Result<AccData> {
    let mut file = File::open(fpath).context("read element data error")?;
    let mut data = Vec::new();
    file.read_to_end(&mut data).context("read element data error")?;
    serde_json::from_slice(&data).context("read element data error")
}

// deleteAccData delete from the given index
pub fn delete_acc_data(dir: &str, last: i32) -> Result<()> {
    let fs = fs::read_dir(dir).context("delete element data error")?;
    for entry in fs {
        let entry = entry.context("delete element data error")?;
        let path = entry.path();
        if let Some(file_name) = path.file_name() {
            if let Some(file_name_str) = file_name.to_str() {
                if let Some(index_str) = file_name_str.rsplit('-').next() {
                    if let Ok(index) = index_str.parse::<i32>() {
                        if index <= last {
                            fs::remove_file(path).context("delete element data error")?;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn clean_backup(dir: &str, index: i64) -> Result<()> {
    let backup = format!("{}/{}-{}", dir, DEFAULT_BACKUP_NAME, index);
    fs::remove_file(backup).context("clean backup error")
}

pub fn backup_acc_data(dir: &str, index: i64) -> Result<()> {
    let fpath = format!("{}/{}-{}", dir, DEFAULT_NAME, index);
    let backup = format!("{}/{}-{}", dir, DEFAULT_BACKUP_NAME, index);
    util::copy_file(&fpath, &backup).context("backup element data error")
}

pub fn backup_acc_data_for_chall(src: &str, des: &str, index: i64) -> Result<()> {
    let fpath = Path::new(src).join(format!("{}-{}", DEFAULT_NAME, index));
    let backup = Path::new(des).join(format!("{}-{}", DEFAULT_NAME, index));
    util::copy_file(fpath.to_str().unwrap(), backup.to_str().unwrap())
        .context("backup acc data for challenge error")?;
    Ok(())
}

pub fn recovery_acc_data(dir: &str, index: i64) -> Result<()> {
    let backup = Path::join(Path::new(dir), &format!("{}-{}", DEFAULT_BACKUP_NAME, index));
    let fpath = Path::join(Path::new(dir), &format!("{}-{}", DEFAULT_NAME, index));

    if !backup.exists() {
        return Ok(());
    }

    fs::rename(&backup, &fpath).context("recovery acc data error")?;
    Ok(())
}
