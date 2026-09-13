use std::io::Write;
use std::path::Path;

pub fn write_json_atomic(path: &Path, data: &serde_json::Value) -> std::io::Result<()> {
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(dir)?;

    let unique = format!(".{}-{}.tmp", std::process::id(), rand::random::<u64>());
    let file_name = format!(
        "{}{}",
        path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
        unique
    );
    let tmp_path = dir.join(file_name);

    let result = (|| -> std::io::Result<()> {
        let bytes = serde_json::to_vec(data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let mut file = std::fs::File::create(&tmp_path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        std::fs::rename(&tmp_path, path)?;
        Ok(())
    })();

    if result.is_err() {
        let _ = std::fs::remove_file(&tmp_path);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_writes_expected_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.json");
        write_json_atomic(&path, &json!({"a": 1})).unwrap();
        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content, json!({"a": 1}));
    }

    #[test]
    fn test_overwrites_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.json");
        write_json_atomic(&path, &json!({"a": 1})).unwrap();
        write_json_atomic(&path, &json!({"a": 2})).unwrap();
        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content, json!({"a": 2}));
    }

    #[test]
    fn test_no_leftover_tmp_files_on_success() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.json");
        write_json_atomic(&path, &json!({"a": 1})).unwrap();
        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp"))
            .collect();
        assert!(leftovers.is_empty());
    }

    #[test]
    fn test_creates_parent_dir_if_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("out.json");
        write_json_atomic(&path, &json!({"a": 1})).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_fails_gracefully_on_unwritable_target_dir() {
        let dir = tempfile::tempdir().unwrap();
        let blocked_file = dir.path().join("blocked");
        std::fs::write(&blocked_file, "x").unwrap();
        let path = blocked_file.join("out.json");
        assert!(write_json_atomic(&path, &json!({"a": 1})).is_err());
    }
}
