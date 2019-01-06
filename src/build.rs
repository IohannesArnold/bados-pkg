use std::fs;
use std::path::{Path, PathBuf};
use errors::*;
use toml;
use sha2::{Sha256, Digest};
use generic_array::GenericArray;
use url::Url;

use structs::*;

#[cfg(target_os = "linux")]
use unshare::{Command, Namespace};
#[cfg(target_os = "linux")]
use libmount::{BindMount};

pub fn start_build(filename: &str, pkg_dir: &Path) -> Result <()>{
    let build_config_string = 
        fs::read_to_string(filename).chain_err(|| "Could not open build config file")?;
    let mut build_conf: BuildConfig = 
        toml::from_str(&build_config_string).chain_err(|| "Could not parse TOML")?;
    let pkg_hash = get_hash::<Sha256>(&build_conf);
    let pkg_triplet = format!("{}-{}-{:x}",
                             &build_conf.pkg_data.name,
                             &build_conf.pkg_data.version,
                             &pkg_hash);
    println!("Building {}", &pkg_triplet);
    let build_dir = Path::new("/tmp").join(&pkg_triplet);
    fs::create_dir(&build_dir)
        .chain_err(|| "Could not create build directory")?;
    mount_deps(&mut build_conf.build_data, &build_dir, &pkg_dir)
        .chain_err(|| "Could not mount all dependencies")?;
    get_src_files(&build_conf.build_data.src_files, &build_dir)
        .chain_err(|| "Could not gather all sources")?;
    let out_dir = setup_outdir(&mut build_conf.build_data, &build_dir, &pkg_triplet)?;
    execute_build(&build_conf.build_data, &build_dir)?;
    install_result(&out_dir, &pkg_dir, &pkg_triplet)?;
    cleanup()
}

fn get_hash<Hasher: Digest> (build_config: &BuildConfig) -> GenericArray<u8, Hasher::OutputSize>{
    let mut hasher = Hasher::default();
    hasher.input(build_config.pkg_data.name.as_bytes());
    hasher.input(build_config.pkg_data.version.as_bytes());
    for pkg_dep in &build_config.pkg_data.pkg_deps {
        hasher.input(&pkg_dep.hash);
    }
    for build_dep in &build_config.build_data.build_deps {
        hasher.input(&build_dep.hash);
    }
    for src_file in &build_config.build_data.src_files {
        hasher.input(&src_file.hash);
    }
    hasher.input(build_config.build_data.build_init.as_bytes());
    for build_var in &build_config.build_data.build_vars {
        hasher.input(build_var.0.as_bytes());
        hasher.input(build_var.1.as_bytes());
    }
    for build_arg in &build_config.build_data.build_args {
        hasher.input(build_arg.as_bytes());
    }
    hasher.result()

}

fn mount_deps(build_data: &mut BuildData, build_dir_org: &PathBuf, pkg_dir_org: &Path) -> Result<()> {
    let mut build_dir = build_dir_org.clone();
    build_dir.push("blank");
    let mut pkg_dir = pkg_dir_org.join("store/blank");
    let path_env_var = build_data.build_vars.entry("PATH").or_default();
    let mut path_env_dir = PathBuf::from("/");
    for ref dep in &build_data.build_deps {
        let dep_concat = format!("{}-{}-{:x}", dep.name, dep.version, dep.hash);
        pkg_dir.set_file_name(&dep_concat);
        build_dir.set_file_name(&dep_concat);
        path_env_dir.set_file_name(&dep_concat);
        fs::create_dir(&build_dir)?;
        BindMount::new(&pkg_dir, &build_dir)
                  .readonly(true)
                  .mount().unwrap();
        path_env_dir.push("bin");
        path_env_var.push_str(&path_env_dir.to_str().unwrap());
        path_env_var.push_str(":");
        path_env_dir.pop();
    }
    Ok(())
}

fn get_src_files(src_files: &Vec<SrcFile>, build_dir: &PathBuf) -> Result<()>{
    for src_file in src_files {
        println!("Fetching {}", src_file.name);
        get_src_file(&src_file, &build_dir)?;
    }
    Ok(())
}
fn get_src_file(src_file: &SrcFile, build_dir: &PathBuf) -> Result<()>{
    let output_name = Path::new(build_dir).join(&src_file.name);
    for url_str in &src_file.urls {
        let url = Url::parse(&url_str).unwrap();
        println!("Fetching {} from {}", &src_file.name, &url);
        let result = match url.scheme() {
            "file" => verified_copy(&url.path(),
                                    &src_file.hash,
                                    &output_name),
            _ => unimplemented!()
        };
        if let Ok(_) = result { return Ok(()) }
        eprintln!("Could not fetch {} from {}", &src_file.name, &url);
    }
    Err(Error::from("None of the sources are sucessful"))
}

fn verified_copy(path: &str, hash: &[u8], output: &PathBuf) -> Result<u64> {
    let mut file = fs::File::open(&path)?;
    let hash_result = Sha256::digest_reader(&mut file)?;
    if hash_result.as_slice() != hash {
        bail!("{} has hash {:x}", &path, &hash_result);
    }
    fs::copy(&path, &output).chain_err(||"Could not copy file")
}

fn setup_outdir(build_data: &mut BuildData, build_dir: &PathBuf, pkg_triplet: &str) -> Result<PathBuf> {
    let mut out_dir = build_dir.clone();
    out_dir.push(&pkg_triplet);
    let mut out_dir_env = String::from("/");
    out_dir_env.push_str(&pkg_triplet);
    fs::create_dir(&out_dir)
        .chain_err(|| "Could not create build output directory")?;
    build_data.build_vars.insert("OUT_DIR", out_dir_env);
    Ok(out_dir)
}

#[cfg(target_os = "linux")]
fn execute_build(build_data: &BuildData, build_dir: &PathBuf) -> Result <()> {
    println!("Executing {}", build_data.build_init);
    let mut command = Command::new(&build_data.build_init);
                            command.unshare(&[Namespace::Mount,
                                       Namespace::Uts,
                                       Namespace::Ipc,
                                       Namespace::Pid,
                                       Namespace::Net,
                                       Namespace::Cgroup])
                            .env_clear()
                            .chroot_dir(&build_dir);
    for (ref env_var_key, ref env_var_val) in &build_data.build_vars {
        println!("{}: {}", env_var_key, env_var_val);
        command.env(env_var_key, env_var_val);
    }
    command.status()
           .chain_err(|| "Unable to sucessfully build")
           .map(|_| ())
}

fn install_result(out_dir: &PathBuf, pkg_dir: &Path, pkg_triplet: &str) -> Result<u64> {
    let mut store_dir = pkg_dir.join("store");
    store_dir.push(&pkg_triplet);
    fs::create_dir(&store_dir).chain_err(|| "Could not create directory")?;
    fs::copy(&out_dir, &store_dir).chain_err(|| "Could not copy file")
}

fn cleanup() -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
}
