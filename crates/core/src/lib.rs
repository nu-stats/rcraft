//! Shared types, limits and error values.

/// The product name shown in titles and `--version`.
pub const PRODUCT: &str = "RCraft";

/// Version of this build.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_the_product() {
        assert_eq!(PRODUCT, "RCraft");
        assert!(!version().is_empty());
    }
}
