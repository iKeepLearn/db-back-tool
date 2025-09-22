use std::fs;
use std::path::PathBuf;

pub fn resolve_path(path_str: &str) -> Result<PathBuf, String> {
    let resolved_path = if path_str.starts_with("~") {
        let expanded_str = shellexpand::tilde(path_str);
        PathBuf::from(expanded_str.to_string())
    } else {
        PathBuf::from(path_str)
    };

    if resolved_path.exists() {
        fs::canonicalize(&resolved_path).map_err(|e| format!("Could not canonicalize path: {}", e))
    } else {
        Ok(resolved_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_resolve_path_existing() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("testfile.txt");
        File::create(&file_path).unwrap();

        let resolved = resolve_path(file_path.to_str().unwrap()).unwrap();
        assert!(resolved.is_absolute());
        assert!(resolved.ends_with("testfile.txt"));
    }

    #[test]
    fn test_resolve_path_non_existing() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("not_exist.txt");

        let resolved = resolve_path(file_path.to_str().unwrap()).unwrap();
        assert_eq!(resolved, file_path);
    }

    #[test]
    fn test_resolve_path_with_tilde() {
        let home = env::var("HOME").unwrap();
        let test_path = "~/testfile";
        let resolved = resolve_path(test_path).unwrap();
        assert!(resolved.starts_with(&home));
        assert!(resolved.ends_with("testfile"));
    }
}
