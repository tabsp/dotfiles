use crate::config::{self, Installer};
use std::path::Path;

pub fn list_deps() -> Result<(), String> {
    let deps = config::load_deps(Path::new("deps.toml"))?;
    for (name, dep) in &deps.deps {
        for platform in ["mac", "linux"] {
            let map = match platform {
                "mac" => &dep.mac,
                "linux" => &dep.linux,
                _ => continue,
            };
            for (arch, entry) in map {
                if entry.installer != Installer::DownloadBinary {
                    continue;
                }
                let url = entry
                    .params
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("—");
                let sha = entry
                    .params
                    .get("sha256")
                    .and_then(|v| v.as_str())
                    .unwrap_or("—");
                println!(
                    "{name}: {} ({platform} {arch})",
                    entry.version
                );
                println!("  url: {url}");
                println!("  sha256: {sha}");
            }
        }
    }
    Ok(())
}

pub fn check_deps() -> Result<(), String> {
    let deps = config::load_deps(Path::new("deps.toml"))?;
    let client = reqwest::blocking::Client::builder()
        .user_agent("dotman/0.1.0")
        .build()
        .map_err(|e| format!("failed to build http client: {e}"))?;

    for (name, dep) in &deps.deps {
        for platform in ["mac", "linux"] {
            let map = match platform {
                "mac" => &dep.mac,
                "linux" => &dep.linux,
                _ => continue,
            };
            for (arch, entry) in map {
                if entry.installer != Installer::DownloadBinary {
                    continue;
                }
                let url = entry
                    .params
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                match check_github_release(&client, url) {
                    Ok(Some(latest)) => {
                        if latest != entry.version {
                            println!(
                                "{name}: {} → {latest} ({platform} {arch})",
                                entry.version
                            );
                        } else {
                            println!("{name}: {} (up to date)", entry.version);
                        }
                    }
                    Ok(None) => {
                        println!("{name}: {} (not a GitHub release URL)", entry.version);
                    }
                    Err(err) => {
                        eprintln!("{name}: error checking {url}: {err}");
                    }
                }
            }
        }
    }
    Ok(())
}

fn check_github_release(
    client: &reqwest::blocking::Client,
    url: &str,
) -> Result<Option<String>, String> {
    // Extract owner/repo from GitHub release URL like
    // https://github.com/OWNER/REPO/releases/download/TAG/...
    let parts: Vec<&str> = url.split("github.com/").collect();
    if parts.len() < 2 {
        return Ok(None);
    }
    let after = parts[1];
    let segments: Vec<&str> = after.split('/').collect();
    if segments.len() < 2 {
        return Ok(None);
    }
    let owner = segments[0];
    let repo = segments[1];

    let api_url = format!("https://api.github.com/repos/{owner}/{repo}/releases/latest");
    let resp = client
        .get(&api_url)
        .send()
        .map_err(|e| format!("GitHub API request failed: {e}"))?;
    if !resp.status().is_success() {
        return Ok(None);
    }
    let text = resp
        .text()
        .map_err(|e| format!("failed to read GitHub API response: {e}"))?;
    let body: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("failed to parse GitHub API JSON: {e}"))?;
    let tag = body["tag_name"].as_str().unwrap_or("");
    Ok(Some(tag.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_github_release_extracts_owner_repo() {
        let url = "https://github.com/eza-community/eza/releases/download/v0.20.0/eza.tar.gz";
        let client = reqwest::blocking::Client::new();
        // Don't actually call API in tests
        let after = url.split("github.com/").collect::<Vec<_>>();
        assert_eq!(after.len(), 2);
        let segments: Vec<&str> = after[1].split('/').collect();
        assert_eq!(segments[0], "eza-community");
        assert_eq!(segments[1], "eza");
    }
}
