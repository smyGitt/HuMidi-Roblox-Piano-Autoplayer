use std::time::Duration;

pub const RELEASES_PAGE: &str =
    "https://github.com/smyGitt/HuMidi-Roblox-Piano-Autoplayer/releases/latest";
const LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/smyGitt/HuMidi-Roblox-Piano-Autoplayer/releases/latest";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const USER_AGENT: &str = "HuMidi-update-check";
const GITHUB_ACCEPT: &str = "application/vnd.github+json";

pub fn parse_version(tag: &str) -> Vec<i64> {
    let stripped = tag.trim_start_matches('v').trim();
    if stripped.is_empty() {
        return Vec::new();
    }
    let mut result = Vec::new();
    for part in stripped.split('.') {
        match part.parse::<i64>() {
            Ok(n) => result.push(n),
            Err(_) => return Vec::new(),
        }
    }
    result
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum UpdateCheckOutcome {
    UpdateAvailable { tag: String, url: String },
    NoUpdate,
    Indeterminate,
}

pub fn evaluate_update(current_version: &str, latest_tag: &str) -> UpdateCheckOutcome {
    if latest_tag.is_empty() {
        return UpdateCheckOutcome::Indeterminate;
    }
    let latest = parse_version(latest_tag);
    let current = parse_version(current_version);
    if latest.is_empty() || current.is_empty() {
        return UpdateCheckOutcome::Indeterminate;
    }
    if latest <= current {
        return UpdateCheckOutcome::NoUpdate;
    }
    UpdateCheckOutcome::UpdateAvailable {
        tag: latest_tag.to_string(),
        url: RELEASES_PAGE.to_string(),
    }
}

pub fn auto_check_enabled(config: &serde_json::Value) -> bool {
    config
        .get("auto_check_updates")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

pub fn latest_tag_from_release(release: &serde_json::Value) -> String {
    release
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

async fn fetch_latest_release() -> Result<serde_json::Value, String> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| e.to_string())?;
    client
        .get(LATEST_RELEASE_API)
        .header(reqwest::header::ACCEPT, GITHUB_ACCEPT)
        .send()
        .await
        .and_then(|response| response.error_for_status())
        .map_err(|e| e.to_string())?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| e.to_string())
}

pub async fn check_for_updates(current_version: &str) -> UpdateCheckOutcome {
    match fetch_latest_release().await {
        Ok(release) => evaluate_update(current_version, &latest_tag_from_release(&release)),
        Err(_) => UpdateCheckOutcome::Indeterminate,
    }
}

#[cfg(test)]
mod tests;
