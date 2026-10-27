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
mod tests;
