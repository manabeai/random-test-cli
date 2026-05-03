use serde_json::Value;
use std::cmp::Ordering;
use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("repository URL is not a GitHub repository: {0}")]
    UnsupportedRepository(String),
    #[error("failed to run {program}: {source}")]
    Command {
        program: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{program} failed with status {status}: {stderr}")]
    CommandFailed {
        program: String,
        status: String,
        stderr: String,
    },
    #[error("failed to parse GitHub release response: {0}")]
    ReleaseJson(#[from] serde_json::Error),
    #[error("latest GitHub release response did not contain a tag_name")]
    MissingTagName,
}

pub fn update_from_github(
    repository: &str,
    installer_name: &str,
    current_version: &str,
) -> Result<(), UpdateError> {
    let repo_path = github_repo_path(repository)?;
    let api_url = format!("https://api.github.com/repos/{repo_path}/releases/latest");
    let release_json = curl_stdout([
        "-fsSL",
        "-H",
        "Accept: application/vnd.github+json",
        "-H",
        "User-Agent: rt-updater",
        &api_url,
    ])?;
    let latest_tag = latest_tag_name(&release_json)?;

    if !should_update(current_version, &latest_tag) {
        println!("rt is already up to date ({current_version}).");
        return Ok(());
    }

    println!("updating rt {current_version} -> {latest_tag}");
    let installer_url = format!(
        "https://github.com/{repo_path}/releases/download/{latest_tag}/{installer_name}-installer.sh"
    );
    let installer_path = std::env::temp_dir().join(format!(
        "{installer_name}-installer-{}.sh",
        std::process::id()
    ));

    curl_to_file(&installer_url, &installer_path)?;
    run_sh(&installer_path)?;
    let _ = std::fs::remove_file(&installer_path);
    Ok(())
}

fn github_repo_path(repository: &str) -> Result<String, UpdateError> {
    let path = repository
        .strip_prefix("https://github.com/")
        .or_else(|| repository.strip_prefix("http://github.com/"))
        .ok_or_else(|| UpdateError::UnsupportedRepository(repository.to_string()))?
        .trim_end_matches(".git")
        .trim_matches('/');

    let mut parts = path.split('/');
    let owner = parts.next().unwrap_or_default();
    let repo = parts.next().unwrap_or_default();
    if owner.is_empty() || repo.is_empty() {
        return Err(UpdateError::UnsupportedRepository(repository.to_string()));
    }
    Ok(format!("{owner}/{repo}"))
}

fn latest_tag_name(json: &[u8]) -> Result<String, UpdateError> {
    let value: Value = serde_json::from_slice(json)?;
    value
        .get("tag_name")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or(UpdateError::MissingTagName)
}

fn should_update(current_version: &str, latest_tag: &str) -> bool {
    match compare_versions(latest_tag, current_version) {
        Some(Ordering::Greater) => true,
        Some(_) => false,
        None => normalize_version(latest_tag) != normalize_version(current_version),
    }
}

fn compare_versions(left: &str, right: &str) -> Option<Ordering> {
    let left = parse_version(left)?;
    let right = parse_version(right)?;
    Some(left.cmp(&right))
}

fn parse_version(version: &str) -> Option<Vec<u64>> {
    let normalized = normalize_version(version);
    let core = normalized.split(['-', '+']).next().unwrap_or(normalized);
    let parts = core
        .split('.')
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    if parts.is_empty() { None } else { Some(parts) }
}

fn normalize_version(version: &str) -> &str {
    version.trim().trim_start_matches('v')
}

fn curl_stdout<I, S>(args: I) -> Result<Vec<u8>, UpdateError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output =
        Command::new("curl")
            .args(args)
            .output()
            .map_err(|source| UpdateError::Command {
                program: "curl".to_string(),
                source,
            })?;
    if !output.status.success() {
        return Err(UpdateError::CommandFailed {
            program: "curl".to_string(),
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(output.stdout)
}

fn curl_to_file(url: &str, path: &Path) -> Result<(), UpdateError> {
    let path = path.to_string_lossy();
    curl_stdout(["-fsSL", "-o", path.as_ref(), url]).map(|_| ())
}

fn run_sh(path: &Path) -> Result<(), UpdateError> {
    let status = Command::new("sh")
        .arg(path)
        .status()
        .map_err(|source| UpdateError::Command {
            program: "sh".to_string(),
            source,
        })?;
    if !status.success() {
        return Err(UpdateError::CommandFailed {
            program: "sh".to_string(),
            status: status.to_string(),
            stderr: String::new(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_github_repo_path() {
        assert_eq!(
            github_repo_path("https://github.com/manabeai/random-test-cli").unwrap(),
            "manabeai/random-test-cli"
        );
        assert_eq!(
            github_repo_path("https://github.com/manabeai/random-test-cli.git").unwrap(),
            "manabeai/random-test-cli"
        );
    }

    #[test]
    fn compares_latest_tag_to_current_version() {
        assert!(!should_update("0.1.0", "v0.1.0"));
        assert!(should_update("0.1.0", "v0.1.1"));
        assert!(!should_update("0.2.0", "v0.1.9"));
    }

    #[test]
    fn reads_latest_release_tag() {
        assert_eq!(
            latest_tag_name(br#"{"tag_name":"v1.2.3"}"#).unwrap(),
            "v1.2.3"
        );
    }
}
