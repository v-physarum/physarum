//! Downstream `Cargo.toml`, serialized with serde.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;

use crate::error::Result;

/// One path dependency of the downstream crate.
pub(crate) struct DepSpec<'a> {
    pub(crate) name: &'a str,
    pub(crate) path: &'a Path,
}

/// Downstream `Cargo.toml` written before a measured build.
pub(crate) struct UserManifest<'a> {
    package: &'a str,
    dependencies: &'a [DepSpec<'a>],
}

impl<'a> UserManifest<'a> {
    /// `package` is the downstream crate. `dependencies` are path dependencies.
    pub(crate) fn new(package: &'a str, dependencies: &'a [DepSpec<'a>]) -> Self {
        Self { package, dependencies }
    }

    /// Serializes this manifest as TOML.
    ///
    /// # Errors
    ///
    /// Returns an error when the document cannot be serialized.
    pub(crate) fn to_toml(&self) -> Result<String> {
        let mut owned = Vec::with_capacity(self.dependencies.len());
        for dep in self.dependencies {
            owned.push((dep.name, dep.path.display().to_string()));
        }
        let mut dependencies = BTreeMap::new();
        for (name, path) in &owned {
            dependencies.insert(*name, PathDep::new(path));
        }
        let document = Document::new(self.package, dependencies);
        Ok(toml::to_string(&document)?)
    }
}

#[derive(Serialize)]
struct Document<'a> {
    workspace: BTreeMap<&'a str, &'a str>,
    package: Package<'a>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    dependencies: BTreeMap<&'a str, PathDep<'a>>,
}

impl<'a> Document<'a> {
    fn new(package: &'a str, dependencies: BTreeMap<&'a str, PathDep<'a>>) -> Self {
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
        let path = Path::new("/tmp/macro");
        let deps = [super::DepSpec { name: "ph_util_foreach", path }];
        let text = UserManifest::new("ph_util_foreach_user", &deps)
            .to_toml()
            .unwrap();
        assert!(text.contains("[workspace]"), "{text}");
        assert!(text.contains("name = \"ph_util_foreach_user\""), "{text}");
        assert!(text.contains("edition = \"2024\""), "{text}");
        assert!(text.contains("publish = false"), "{text}");
        assert!(text.contains("ph_util_foreach"), "{text}");
        assert!(text.contains("/tmp/macro"), "{text}");
    }

    #[test]
    fn an_empty_dependency_list_omits_the_table() {
        let text = UserManifest::new("hand_structs_user", &[])
            .to_toml()
            .unwrap();
        assert!(!text.contains("[dependencies]"), "{text}");
        assert!(text.contains("[workspace]"), "{text}");
    }
}
