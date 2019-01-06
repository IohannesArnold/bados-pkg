#![recursion_limit = "1024"]
#[macro_use]
extern crate clap;
#[macro_use]
extern crate error_chain;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate url;
/*#[macro_use]
extern crate diesel;*/
#[cfg(target_os = "linux")]
extern crate unshare;
#[cfg(target_os = "linux")]
extern crate libmount;
extern crate sha2;
extern crate digest;
extern crate generic_array;
extern crate flate2;
extern crate tar;
extern crate toml;
//extern crate libc;

use std::path::Path; 
use std::fs::create_dir;
//use std::str;

use clap::{App,SubCommand,Arg,ArgGroup};

mod errors { error_chain!{
    foreign_links { 
        Io(::std::io::Error);
    }
} }
use errors::*;
mod defaults;
use defaults::*;
mod install;
use install::*;
mod build;
use build::*;
mod structs;

fn main() {
    let args = App::new("bados-pkg")
                   .version(crate_version!())
                   .arg(Arg::with_name(PKGDIR_NAME)
                        .global(true)
                        .help(PKGDIR_HELP)
                        .long(PKGDIR_NAME)
                        .takes_value(true)
                        .value_name("DIR"))
                   .subcommand(SubCommand::with_name(INIT_NAME)
                               .about("Initializes the application database and structure"))
                   .subcommand(SubCommand::with_name("build")
                               .arg(Arg::with_name(FROM_CONFIG_NAME)
                                    .long(FROM_CONFIG_NAME)
                                    .takes_value(true)
                                    .value_name("CONFIG")))
                   .subcommand(SubCommand::with_name(INSTALL_NAME)
                               .about("Installs a package")
                               .arg(Arg::with_name("set-version")
                                    .long("set-version")
                                    .takes_value(true)
                                    .value_name("VERSION"))
                               .arg(Arg::with_name("from-repo")
                                    .takes_value(true)
                                    .value_name("PACKAGE"))
                               .arg(Arg::with_name(COPY_FILE_NAME)
                                    .long(COPY_FILE_NAME)
                                    .help(COPY_FILE_HELP)
                                    .takes_value(true)
                                    .value_name(FILE_UPPER))
                               .arg(Arg::with_name("from-archive")
                                    .long("from-archive")
                                    .takes_value(true)
                                    .value_name(FILE_UPPER))
                               .group(ArgGroup::with_name("install_type")
                                      .args(&[COPY_FILE_NAME, "from-repo", "from-archive"])
                                      .required(true)))

                 .get_matches();
    let pkgdir_parent = args.value_of(PKGDIR_NAME);
    let outcome = match args.subcommand() {
        (INIT_NAME, Some(subcmd)) => { 
            let pkgdir = Path::new(subcmd.value_of(PKGDIR_NAME).or(pkgdir_parent).unwrap_or("/pkg"));
            initialize_pkgdir(pkgdir)
        },
        (INSTALL_NAME, Some(subcmd)) => { 
            let pkgdir = Path::new(subcmd.value_of(PKGDIR_NAME).or(pkgdir_parent).unwrap_or("/pkg"));
            let pkg_version = subcmd.value_of("set-version").unwrap_or("0.0.0");
            if let Some(filename) = subcmd.value_of(COPY_FILE_NAME) {
                install_copy_file(pkgdir, filename, pkg_version).map(|_|())
            } else if let Some(filename) = subcmd.value_of("from-archive") {
                install_from_tarball(pkgdir, filename, pkg_version)
            } else if let Some(pkg_name) = subcmd.value_of("from-repo") {
                install_from_repo(pkgdir, pkg_name)
            } else {
                unreachable!();
            }
        },
        ("build", Some(subcmd)) => {
            let pkgdir = Path::new(subcmd.value_of(PKGDIR_NAME).or(pkgdir_parent).unwrap_or("/pkg"));
            let filename = subcmd.value_of(FROM_CONFIG_NAME).unwrap_or("./Build.toml");
            start_build(&filename, &pkgdir)
        },
        _ => {Ok(())}
    };
    if let Err(ref e) = outcome {
        eprintln!("error: {}", e);
        for e in e.iter().skip(1) {
            eprintln!("caused by: {}", e);
        }
        ::std::process::exit(1);
    }
}

fn initialize_pkgdir(pkgdir: &Path) -> Result<()> {
    println!("Initializing at {}", pkgdir.display());
    if !pkgdir.is_dir() {
        bail!("Cannot read directory or was passed a file")
    }
    ["store", "src"].iter()
                    .try_for_each(|x| create_dir(pkgdir.join(x))) 
                    .chain_err(|| format!("{} not sucessfully initialized", pkgdir.display()))
}

