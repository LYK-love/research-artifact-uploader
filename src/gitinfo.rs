use std::path::Path;
use std::process::Command;

use crate::AppResult;

#[derive(Debug)]
pub struct GitInfo {
    pub available: bool,
    pub repo_root: Option<String>,
    pub commit: Option<String>,
    pub branch: Option<String>,
    pub dirty: Option<bool>,
}

pub fn read_git_info() -> GitInfo {
    let root = run_git(["rev-parse", "--show-toplevel"].as_slice())
        .and_then(|s| Ok(s.trim().to_string()))
        .ok();
    if root.is_none() {
        return GitInfo {
            available: false,
            repo_root: None,
            commit: None,
            branch: None,
            dirty: None,
        };
    }

    let commit = run_git(["rev-parse", "HEAD"].as_slice()).ok();
    let branch = run_git(["branch", "--show-current"].as_slice()).ok();
    let dirty = run_git(["status", "--porcelain"].as_slice())
        .ok()
        .map(|s| !s.trim().is_empty());

    GitInfo {
        available: true,
        repo_root: root,
        commit: commit.map(|v| v.trim().to_string()),
        branch: branch.map(|v| v.trim().to_string()),
        dirty,
    }
}

fn run_git(args: &[&str]) -> AppResult<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(Path::new("."))
        .output()
        .map_err(|err| format!("git command failed: {err}"))?;
    if !out.status.success() {
        return Err("git command failed".to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}
