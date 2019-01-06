use std::collections::HashMap;
use sha2::Sha256;
use digest::FixedOutput;
use generic_array::GenericArray;
type Sha256Array = GenericArray<u8, <Sha256 as FixedOutput>::OutputSize>;

#[derive(Deserialize)]
pub struct BuildConfig<'a> {
    #[serde(borrow)]
    pub pkg_data: PkgData<'a>,
    #[serde(borrow)]
    pub build_data: BuildData<'a>,
}

#[derive(Deserialize)]
pub struct PkgData<'a> {
    pub name: &'a str,
    pub version: &'a str,
    #[serde(borrow)]
    #[serde(default = "Vec::new")]
    pub pkg_deps: Vec<Dep<'a>>,
}

#[derive(Deserialize)]
pub struct BuildData<'a> {
    #[serde(borrow)]
    #[serde(default = "Vec::new")]
    pub build_deps: Vec<Dep<'a>>,
    #[serde(borrow)]
    pub src_files: Vec<SrcFile<'a>>,
    #[serde(default = "HashMap::new")]
    pub build_vars: HashMap<&'a str, String>,
    pub build_init: &'a str,
    #[serde(borrow)]
    #[serde(default = "Vec::new")]
    pub build_args: Vec<&'a str>
}

#[derive(Deserialize, Copy, Clone)]
pub struct Dep<'a> {
    pub name: &'a str,
    pub version: &'a str,
    #[serde(deserialize_with = "hash_str_to_array")]
    pub hash: Sha256Array,
}

#[derive(Deserialize)]
pub struct SrcFile<'a> {
    pub name: &'a str,
    #[serde(deserialize_with = "hash_str_to_array")]
    pub hash: Sha256Array,
    #[serde(borrow)]
    pub urls: Vec<&'a str>,
}

use serde::de::{Deserializer, Visitor, Error};
use std::fmt;
struct HashStrVisitor;

impl<'de> Visitor<'de> for HashStrVisitor {
    type Value = Sha256Array;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a hexadecimal string")
    }

    fn visit_borrowed_str<E>(self, v:&'de str) -> Result<Self::Value, E>
    where E: Error, {
        let generator_closure = |index| -> u8 {
            let pair = v.get(index*2..2*index+2).unwrap();
            u8::from_str_radix(&pair, 16).unwrap()
        };
        let ga = GenericArray::generate(&generator_closure);
        Ok(ga)
    }
}

fn hash_str_to_array<'de, D>(d: D) -> Result<Sha256Array, D::Error>
where D: Deserializer<'de> {
    d.deserialize_str(HashStrVisitor)
}

/*
// This is an attempt to make a more generic array deserialization
// It has not been successfull

struct HashNewType<H: Digest>(
    GenericArray<u8, H::OutputSize>
);

struct HashStrVisitor<H:Digest>(H);

impl<'de, HashInner> Visitor<'de> for HashStrVisitor<HashInner>
where HashInner: Digest{
    type Value = HashNewType<HashInner>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a hexadecimal string")
    }

    fn visit_borrowed_str<E>(self, v:&'de str) -> Result<Self::Value, E>
    where E: Error, {
        let generator_closure = |index| -> u8 {
            let pair = v.get(index..index+2).unwrap();
            u8::from_str_radix(&pair, 16).unwrap()
        };
        let ga: GenericArray<u8, HashInner::OutputSize> =
            GenericArray::generate(generator_closure);
        Ok(HashNewType(ga))
    }
}

impl<'de, HashOuter> Deserialize<'de> for HashNewType<HashOuter> 
where HashOuter: Digest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        let visitor = HashStrVisitor::<HashOuter>;
        deserializer.deserialize_str(visitor)
    }
}

#[derive(Deserialize)]
enum HashType {
    sha256_hash(HashNewType<Sha256>),
    sha516_hash(HashNewType<Sha512>),
}
*/

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest};
    use toml;

    const TEST_TOML: &str = r#"
    [pkg_data]
    name = 'hello world'
    version = '3.3.3'
    [build_data]
    build_init = './hi.sh'
    [[build_data.src_files]]
    name = 'hello'
    hash='2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824'
    urls=['file:///hello']
    "#;

    #[test]
    fn test_hash_deserialization() {
        let build_config: BuildConfig = toml::from_str(TEST_TOML).unwrap();
        let hello_hash = Sha256::digest_str("hello");
        assert_eq!(build_config.build_data.src_files[0].hash, hello_hash);
    }
    #[test]
    fn test_toml_parsing() {
        let build_config: BuildConfig = toml::from_str(TEST_TOML).unwrap();
        assert_eq!("hello world", build_config.pkg_data.name);
    }
}
