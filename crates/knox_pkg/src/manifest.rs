//! knox.toml manifest parsing.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::io;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub dependencies: BTreeMap<String, Dependency>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Dependency {
    Path { path: String },
}

#[allow(dead_code)]
pub fn load_manifest(path: &Path) -> io::Result<Manifest> {
    let s = std::fs::read_to_string(path)?;
    toml::from_str(&s).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}
