//! The built `ph_beans` rlib, for probes compiled outside this test binary.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Crate name the probes import.
pub const CRATE: &str = "ph_beans";
const LIBRARY_PREFIX: &str = "libph_beans-";
const LIBRARY_EXTENSION: &str = "rlib";
const OUT_DIRECTORY: &str = "out";

/// Newest `ph_beans` rlib next to this test binary.
///
/// Cargo puts it in the binary's own directory, or in a sibling
/// `<hash>/out` directory of the same package.
pub fn library() -> PathBuf {
    let exe = std::env::current_exe().expect("test binary path");
    let own = exe.parent().expect("test binary directory");
    let mut found = rlibs(own);
    let package = own.parent().and_then(Path::parent);
    for unit in package
        .and_then(|dir| dir.read_dir().ok())
        .into_iter()
        .flatten()
        .flatten()
    {
        found.extend(rlibs(&unit.path().join(OUT_DIRECTORY)));
    }
    found
        .into_iter()
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
        .expect("a built ph_beans rlib")
}

fn rlibs(dir: &Path) -> Vec<(SystemTime, PathBuf)> {
    let mut found = Vec::new();
    for entry in dir.read_dir().into_iter().flatten().flatten() {
        let path = entry.path();
        let named = entry
            .file_name()
            .to_string_lossy()
            .starts_with(LIBRARY_PREFIX);
        let rlib = path.extension().is_some_and(|ext| ext == LIBRARY_EXTENSION);
        if let (true, true, Ok(modified)) =
            (named, rlib, entry.metadata().and_then(|m| m.modified()))
        {
            found.push((modified, path));
        }
    }
    found
}
