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
