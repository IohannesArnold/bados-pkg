use std::fs;
use std::fs::{create_dir, File};
use std::path::Path; 
use sha2::{Sha256, Digest};
use flate2::read::GzDecoder;
use tar::Archive;
use errors::*;

pub fn install_copy_file(pkgdir: &Path, filename: &str, pkg_version: &str) -> Result<u64> {
    let file_path = Path::new(filename);
    let mut store_dir = Path::new(pkgdir).join("store");
    if !file_path.is_file() {
        bail!("Cannot read file or was passed a directory")
    }
    let mut file = File::open(filename).chain_err(|| "Could not open file")?;
    let hash = Sha256::digest_reader(&mut file).chain_err(|| "Could not hash file")?;
    let pkg_name = file_path.file_stem().unwrap().to_string_lossy();
    let full_name = format!("{}-{}-{:x}", &pkg_name, &pkg_version, &hash);
    println!("Installing {}", &full_name);
    store_dir.push(full_name);
    create_dir(&store_dir).chain_err(|| "Could not create directory")?;
    store_dir.push(&file_path.file_name().unwrap());
    fs::copy(&file_path, &store_dir).chain_err(|| "Could not copy file")
}

pub fn install_from_tarball(pkgdir: &Path, filename: &str, pkg_version: &str) -> Result<()> {
    let file_path = Path::new(filename);
    let mut store_dir = Path::new(pkgdir).join("store");
    if !file_path.is_file() {
        bail!("Cannot read file or was passed a directory")
    }
    let mut file = File::open(filename).chain_err(|| "Could not open file")?;
    let hash = Sha256::digest_reader(&mut file).chain_err(|| "Could not hash file")?;
    let pkg_name = file_path.file_stem().unwrap().to_string_lossy();
    let full_name = format!("{}-{}-{:x}", &pkg_name, &pkg_version, &hash);
    println!("Installing {}", &full_name);
    store_dir.push(full_name);
    create_dir(&store_dir).chain_err(|| "Could not create directory")?;
    let file2 = File::open(filename).chain_err(|| "Could not open file")?;
    let decoder = GzDecoder::new(file2);
    let mut archive = Archive::new(decoder);
    archive.unpack(&store_dir).chain_err(|| "Could not extract archive")
}

pub fn install_from_repo(pkgdir: &Path, pkg_name: &str) -> Result<()> {
    println!("Installing {}", &pkg_name);
    println!("Not implemented!");
    Ok(())
}

