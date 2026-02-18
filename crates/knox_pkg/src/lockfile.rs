//! knox.lock lockfile: stub that records resolved local deps.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct Lockfile {
    pub version: u32,
    pub packages: BTreeMap<String, ResolvedDep>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResolvedDep {
    pub path: String,
    pub version: String,
}

pub fn load_lockfile(path: &Path) -> io::Result<Lockfile> {
    let s = std::fs::read_to_string(path)?;
    toml::from_str(&s).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Stub: generate lockfile from manifest (resolve local path deps only).
pub fn generate_lockfile(manifest: &super::Manifest) -> Lockfile {
    let mut packages = BTreeMap::new();
    for (name, dep) in &manifest.dependencies {
        let super::Dependency::Path { path } = dep;
        packages.insert(
            name.clone(),
            ResolvedDep {
                path: path.clone(),
                version: "0.1.0".to_string(),
            },
        );
    }
    Lockfile {
        version: 1,
        packages,
    }
}
