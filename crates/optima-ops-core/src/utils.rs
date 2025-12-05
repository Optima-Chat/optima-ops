//! Utility functions

/// Expand tilde (~) to home directory in path strings
pub fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return path.replacen("~", home.to_str().unwrap(), 1);
        }
    }
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_tilde() {
        // Test with tilde
        let path = "~/test/path";
        let expanded = expand_tilde(path);
        assert!(!expanded.starts_with("~"));
        assert!(expanded.ends_with("/test/path"));

        // Test without tilde
        let path = "/absolute/path";
        assert_eq!(expand_tilde(path), "/absolute/path");
    }
}
