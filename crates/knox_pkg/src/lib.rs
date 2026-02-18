//! Knox package manager: manifest (knox.toml), lockfile (knox.lock), local path deps.

mod lockfile;
mod manifest;

pub use lockfile::*;
pub use manifest::{Dependency, Manifest};
