//! Downstream `Cargo.toml`, serialized with serde.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;

use crate::error::Result;

/// Downstream `Cargo.toml` written before a measured build.
pub(crate) struct UserManifest<'a> {
    package: &'a str,
    dependency: &'a str,
    path: &'a Path,
}

impl<'a> UserManifest<'a> {
    /// `package` is the downstream crate. `dependency` is the path dependency's name.
    pub(crate) fn new(package: &'a str, dependency: &'a str, path: &'a Path) -> Self {
        Self { package, dependency, path }
    }

    /// Serializes this manifest as TOML.
    ///
    /// # Errors
    ///
    /// Returns an error when the document cannot be serialized.
    pub(crate) fn to_toml(&self) -> Result<String> {
        let path = self.path.display().to_string();
        let document = Document::new(self.package, self.dependency, &path);
        Ok(toml::to_string(&document)?)
    }
}

#[derive(Serialize)]
struct Document<'a> {
    workspace: BTreeMap<&'a str, &'a str>,
    package: Package<'a>,
    dependencies: BTreeMap<&'a str, PathDep<'a>>,
}

impl<'a> Document<'a> {
    fn new(package: &'a str, dependency: &'a str, path: &'a str) -> Self {
        let mut dependencies = BTreeMap::new();
        dependencies.insert(dependency, PathDep::new(path));
        Self { workspace: BTreeMap::new(), package: Package::new(package), dependencies }
    }
}

#[derive(Serialize)]
struct Package<'a> {
    name: &'a str,
    version: &'static str,
    edition: &'static str,
    publish: bool,
}

impl<'a> Package<'a> {
    fn new(name: &'a str) -> Self {
        Self { name, version: "0.1.0", edition: "2024", publish: false }
    }
}

#[derive(Serialize)]
struct PathDep<'a> {
    path: &'a str,
}

impl<'a> PathDep<'a> {
    fn new(path: &'a str) -> Self {
        Self { path }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::UserManifest;

    #[test]
    fn writes_a_workspace_package_and_path_dependency() {
        let text = UserManifest::new(
            "ph_util_foreach_user",
            "ph_util_foreach",
            Path::new("/tmp/macro"),
        )
        .to_toml()
        .unwrap();
        assert!(text.contains("[workspace]"), "{text}");
        assert!(text.contains("name = \"ph_util_foreach_user\""), "{text}");
        assert!(text.contains("edition = \"2024\""), "{text}");
        assert!(text.contains("publish = false"), "{text}");
        assert!(text.contains("ph_util_foreach"), "{text}");
        assert!(text.contains("/tmp/macro"), "{text}");
    }
}
