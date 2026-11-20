//! TODO: the owner fills in the crate purpose.

/// Package name of this crate.
#[must_use]
pub const fn package_name() -> &'static str {
    "physarum"
}

#[cfg(test)]
mod tests {
    use super::package_name;

    #[test]
    fn package_name_is_physarum() {
        assert_eq!(package_name(), "physarum");
    }
}
