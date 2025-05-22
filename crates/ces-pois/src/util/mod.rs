use std::{
    env::current_dir,
    fs,
    io::{BufReader, BufWriter, Read, Write},
    path::Path,
};

use crate::acc::{self, RsaKey};
use anyhow::{anyhow, bail, Context, Result};
use num_bigint_dig::BigUint;

#[cfg(feature = "use-sysinfo")]
use sysinfo::Disks;

pub fn save_proof_file(path: &Path, data: &[Vec<u8>]) -> Result<()> {
    let f = fs::File::create(path).context("save proof file error")?;
    let mut writer = BufWriter::new(f);

    for d in data {
        let n = writer.write(d)?;
        if n != d.len() {
            bail!("write proof file error:write label error")
        }
    }

    writer.flush()?;
    Ok(())
}

#[cfg(feature = "use-sysinfo")]
pub fn get_dir_free_space(dir: &str) -> Result<u64> {
    let current_dir = current_dir()?;
    let mut dir = Path::new(dir);
    let joined_dir = current_dir.join(dir);

    dir = if dir.is_absolute() { dir } else { &joined_dir };

    let disks = Disks::new_with_refreshed_list();
    let mut available_space = 0;

    for disk in disks.list() {
        if dir.starts_with(disk.mount_point().to_path_buf()) {
            available_space = disk.available_space();
            break;
        }
    }
    Ok(available_space)
}

#[cfg(not(feature = "use-sysinfo"))]
pub fn get_dir_free_space(_dir: &str) -> Result<u64> {
    Err(anyhow!("get_dir_free_space is not available without 'use-sysinfo' feature"))
}

pub fn read_proof_file(path: &Path, num: usize, len: usize) -> Result<Vec<Vec<u8>>> {
    if num <= 0 {
        bail!("illegal label number")
    }

    let file = fs::File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut data = Vec::with_capacity(num);

    for _ in 0..num {
        let mut label = vec![0; len];
        let n = reader.read(&mut label)?;
        if n != len {
            bail!("read label error: expected {} bytes, got {}", len, n)
        }
        data.push(label);
    }

    Ok(data)
}

pub fn delete_dir(dir: &str) -> Result<()> {
    fs::remove_dir_all(dir).context("delete dir error")
}

pub fn save_file(path: &Path, data: &[u8]) -> Result<()> {
    let f = fs::File::create(path)?;
    let mut writer = BufWriter::new(f);

    writer.write_all(data)?;
    writer.flush()?;

    Ok(())
}

pub fn delete_file(path: &str) -> Result<()> {
    if Path::new(path).exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn read_file_to_buf(path: &Path, buf: &mut [u8]) -> Result<()> {
    if buf.is_empty() {
        return Ok(());
    }
    let mut file = fs::File::open(path)?;
    let bytes_read = file.read(buf)?;
    if bytes_read != buf.len() {
        bail!("byte number read does not match")
    }
    Ok(())
}

pub fn copy_data(target: &mut [u8], src: &[&[u8]]) {
    let mut count = 0;
    let lens = target.len();

    for d in src {
        let l = d.len();
        if l == 0 || l + count > lens {
            continue;
        }
        target[count..count + l].copy_from_slice(d);
        count += l;
    }
}

pub fn parse_key(path: &str) -> Result<RsaKey, std::io::Error> {
    let data = fs::read(path)?;
    Ok(get_key_from_bytes(&data))
}

fn get_key_from_bytes(data: &[u8]) -> RsaKey {
    if data.len() < 8 {
        return acc::rsa_keygen(2048);
    }
    let nl = u64::from_be_bytes(data[..8].try_into().unwrap());
    let gl = u64::from_be_bytes(data[8..16].try_into().unwrap());
    if nl == 0 || gl == 0 || data.len() - 16 != (nl + gl) as usize {
        return acc::rsa_keygen(2048);
    }
    let n = BigUint::from_bytes_be(&data[16..16 + nl as usize]);
    let g = BigUint::from_bytes_be(&data[16 + nl as usize..]);
    RsaKey::new(n, g)
}

pub fn add_data(target: &mut [u8], src: &[&[u8]]) {
    let target_len = target.len();
    for s in src {
        if s.len() < target_len {
            continue;
        }
        for (i, elem) in target.iter_mut().enumerate() {
            *elem ^= s[i];
        }
    }
}

pub fn clear_data(target: &mut [u8]) {
    for element in target.iter_mut() {
        *element = 0;
    }
}

pub fn copy_files(src: &str, dst: &str) -> Result<()> {
    if !fs::read_dir(dst).is_err() {
        fs::remove_dir_all(dst)?;
    }

    fs::create_dir_all(dst)?;

    let files = fs::read_dir(src)?;

    //check file in src directory is folder or not , if is folder then continue, otherwise open the file and copy on into det directory
    for file in files {
        let file_path = file?.path();
        if file_path.is_dir() {
            continue;
        } else {
            fs::copy(
                &file_path,
                Path::new(dst).join(
                    file_path
                        .file_name()
                        .ok_or_else(|| anyhow!("Invalid file name"))?
                        .to_str()
                        .unwrap(),
                ),
            )?;
        }
    }

    // fs::copy(src, dst)?;

    Ok(())
}

pub fn copy_file(src: &str, des: &str) -> Result<()> {
    let mut df = fs::File::create(des)?;
    let mut sf = fs::File::open(src)?;

    std::io::copy(&mut sf, &mut df)?;
    df.flush()?;
    Ok(())
}
