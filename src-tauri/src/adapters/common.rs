use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::domain::ScanSummary;

pub fn first_existing(paths: &[PathBuf]) -> Option<PathBuf> {
    paths.iter().find(|path| path.exists()).cloned()
}

pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

pub fn command_version(command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let raw = stdout
        .lines()
        .chain(stderr.lines())
        .find(|line| !line.trim().is_empty())?;
    Some(raw.trim().to_string())
}

pub fn env_ref_keys(env: &HashMap<String, String>) -> Vec<String> {
    let mut keys = env.keys().cloned().collect::<Vec<_>>();
    keys.sort();
    keys
}

pub fn duplicate_name_warnings(
    agent: &str,
    names: impl IntoIterator<Item = String>,
) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut warnings = Vec::new();
    for name in names {
        let normalized = name.trim().to_ascii_lowercase();
        if !seen.insert(normalized.clone()) {
            warnings.push(format!(
                "{agent} has duplicate MCP server name after normalization: {normalized}"
            ));
        }
    }
    warnings
}

pub fn display_path(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

pub fn summary(
    mcp_count: usize,
    skill_count: usize,
    sub_agent_count: usize,
    pi_resource_count: usize,
    errors: Vec<String>,
) -> ScanSummary {
    ScanSummary {
        total_resources: (mcp_count + skill_count + sub_agent_count + pi_resource_count) as u32,
        mcp_count: mcp_count as u32,
        skill_count: skill_count as u32,
        sub_agent_count: sub_agent_count as u32,
        pi_resource_count: pi_resource_count as u32,
        errors,
    }
}
