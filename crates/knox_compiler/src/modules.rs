//! Module path resolution: internal (src/) and external (dependencies).

use knox_pkg::Manifest;
use std::path::{Path, PathBuf};

/// Module path as segments, e.g. ["auth", "token"] for auth::token.
pub type ModPath = Vec<String>;

/// Resolve an internal module path to a source file under package root.
/// e.g. ["auth", "token"] -> src/auth/token.kx
pub fn resolve_internal(package_root: &Path, mod_path: &ModPath) -> Option<PathBuf> {
    if mod_path.is_empty() {
        return None;
    }
    let mut p = package_root.join("src");
    for seg in mod_path {
        p = p.join(seg);
    }
    p.set_extension("kx");
    if p.exists() {
        Some(p)
    } else {
        None
    }
}

/// Resolve external dependency: first segment is dep name, rest is path inside dep.
/// e.g. ["http", "Client"] with dep http = { path = "../http" } -> ../http/src/lib.kx or ../http/src/Client.kx
/// Convention: external package entry is src/lib.kx (module name = last segment of path or "lib").
pub fn resolve_external(
    package_root: &Path,
    manifest: &Manifest,
    mod_path: &ModPath,
) -> Option<PathBuf> {
    let (dep_name, rest) = mod_path.split_first()?;
    let dep = manifest.dependencies.get(dep_name)?;
    let knox_pkg::Dependency::Path { path } = dep;
    let dep_root = package_root.join(path);
    if rest.is_empty() {
        let lib = dep_root.join("src").join("lib.kx");
        if lib.exists() {
            return Some(lib);
        }
        return None;
    }
    let mut p = dep_root.join("src");
    for seg in rest {
        p = p.join(seg);
    }
    p.set_extension("kx");
    if p.exists() {
        Some(p)
    } else {
        None
    }
}

/// Resolve import path: try internal first, then external.
pub fn resolve_import(
    package_root: &Path,
    manifest: Option<&Manifest>,
    mod_path: &ModPath,
) -> Option<PathBuf> {
    resolve_internal(package_root, mod_path)
        .or_else(|| manifest.and_then(|m| resolve_external(package_root, m, mod_path)))
}

/// Get module path from file path under src/. e.g. src/auth/token.kx -> ["auth", "token"].
pub fn file_to_mod_path(src_root: &Path, file_path: &Path) -> Option<ModPath> {
    let rel = file_path.strip_prefix(src_root).ok()?;
    let mut segs: Vec<String> = rel
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .map(|s| s.trim_end_matches(".kx").to_string())
        .collect();
    if segs.last().map(|s| s.as_str()) == Some("lib") {
        segs.pop();
    }
    if segs.is_empty() {
        return None;
    }
    Some(segs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_to_mod_path() {
        let src = Path::new("/p/src");
        assert_eq!(
            file_to_mod_path(src, Path::new("/p/src/user.kx")),
            Some(vec!["user".to_string()])
        );
        assert_eq!(
            file_to_mod_path(src, Path::new("/p/src/auth/token.kx")),
            Some(vec!["auth".to_string(), "token".to_string()])
        );
    }
}
