use std::path::Path;
use regex::Regex;

use crate::AppResult;

pub const ARTIFACT_NAME_RE: &str = r"^[A-Za-z0-9_.-]+$";
pub const DEFAULT_EXCLUDED_DIRS: [&str; 3] = [".git", ".rau", "__pycache__"];

pub fn validate_artifact_name(name: &str) -> AppResult<()> {
    let re = Regex::new(ARTIFACT_NAME_RE).map_err(|err| format!("invalid regex: {err}"))?;
    if !re.is_match(name) {
        return Err(format!(
            "artifact name '{name}' is invalid; allowed chars are A-Z a-z 0-9 _ . -"
        ));
    }
    Ok(())
}

pub fn should_skip_default_excluded(path_text: &str, candidate: &Path, root: &Path) -> bool {
    let explicit_root = first_component(path_text);
    if DEFAULT_EXCLUDED_DIRS.contains(&explicit_root) {
        return false;
    }

    let relative = match candidate.strip_prefix(root) {
        Ok(r) => r,
        Err(_) => return false,
    };

    for part in relative.components() {
        let seg = part.as_os_str().to_string_lossy();
        if DEFAULT_EXCLUDED_DIRS.contains(&seg.as_ref()) {
            return true;
        }
    }
    false
}

fn first_component(path_text: &str) -> &str {
    let normalized = Path::new(path_text)
        .components()
        .next()
        .and_then(|c| c.as_os_str().to_str())
        .unwrap_or("");
    normalized
}
