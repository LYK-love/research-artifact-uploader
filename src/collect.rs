use std::{collections::HashSet, fs, path::{Path, PathBuf}};

use glob::glob;

use crate::{AppResult, manifest::Manifest, paths::{validate_artifact_name, should_skip_default_excluded}};

#[derive(Debug, Clone)]
pub struct CollectedArtifact {
    pub name: String,
    pub artifact_type: String,
    pub source_path: String,
    pub required: bool,
    pub status: ArtifactStatus,
    pub matched_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactStatus {
    Included,
    Missing,
}

impl CollectedArtifact {
    pub fn matched_count(&self) -> usize {
        self.matched_paths.len()
    }
}

pub fn collect_artifacts(
    manifest: &Manifest,
    project_root: &Path,
    allow_outside_project: bool,
) -> AppResult<(Vec<CollectedArtifact>, Vec<String>)> {
    let mut collected = Vec::new();
    let mut warnings = Vec::new();

    for artifact in manifest.artifacts.iter() {
        validate_artifact_name(&artifact.name)?;

        let path_text = artifact.path.as_str();
        let source = if Path::new(path_text).is_absolute() {
            PathBuf::from(path_text)
        } else {
            project_root.join(path_text)
        };

        if artifact.artifact_type != "glob" {
            if !allow_outside_project {
                if !is_within_root(&source, project_root) {
                    return Err(format!(
                        "artifact path '{}' is outside current working directory",
                        artifact.path
                    ));
                }
            }
            if should_skip_default_excluded(path_text, &source, project_root) {
                warnings.push(format!(
                    "artifact '{}' is in default-excluded path and will not be packaged",
                    artifact.path
                ));
                continue;
            }
        }

        let (status, matches) = match artifact.artifact_type.as_str() {
            "file" => collect_file(path_text, &source, artifact.required, &mut warnings)?,
            "directory" => collect_directory(path_text, &source, artifact.required, &mut warnings)?,
            "glob" => collect_glob(
                path_text,
                project_root,
                &source,
                artifact.required,
                allow_outside_project,
                &mut warnings,
            )?,
            _ => {
                return Err(format!(
                    "unsupported artifact type '{}'",
                    artifact.artifact_type
                ))
            }
        };

        collected.push(CollectedArtifact {
            name: artifact.name.clone(),
            artifact_type: artifact.artifact_type.clone(),
            source_path: artifact.path.clone(),
            required: artifact.required,
            status,
            matched_paths: matches,
        });
    }

    Ok((collected, warnings))
}

fn collect_file(path_text: &str, source: &Path, required: bool, warnings: &mut Vec<String>) -> AppResult<(ArtifactStatus, Vec<String>)> {
    if !source.exists() {
        if required {
            return Err(format!("required file artifact missing: {path_text}"));
        }
        warnings.push(format!("optional file artifact missing: {path_text}"));
        return Ok((ArtifactStatus::Missing, Vec::new()));
    }
    if !source.is_file() {
        return Err(format!("file artifact must point to a regular file: {path_text}"));
    }
    Ok((ArtifactStatus::Included, vec![source.to_string_lossy().to_string()]))
}

fn collect_directory(path_text: &str, source: &Path, required: bool, warnings: &mut Vec<String>) -> AppResult<(ArtifactStatus, Vec<String>)> {
    if !source.exists() {
        if required {
            return Err(format!("required directory artifact missing: {path_text}"));
        }
        warnings.push(format!("optional directory artifact missing: {path_text}"));
        return Ok((ArtifactStatus::Missing, Vec::new()));
    }
    if !source.is_dir() {
        return Err(format!("directory artifact must point to a directory: {path_text}"));
    }
    Ok((ArtifactStatus::Included, vec![source.to_string_lossy().to_string()]))
}

fn collect_glob(
    path_text: &str,
    project_root: &Path,
    source: &Path,
    required: bool,
    allow_outside_project: bool,
    warnings: &mut Vec<String>,
) -> AppResult<(ArtifactStatus, Vec<String>)> {
    let pattern = if source.is_absolute() {
        source.to_string_lossy().to_string()
    } else {
        project_root.join(source).to_string_lossy().to_string()
    };

    let mut candidates: Vec<PathBuf> = glob(&pattern)
        .map_err(|err| format!("invalid glob pattern '{}': {err}", path_text))?
        .filter_map(Result::ok)
        .collect();

    candidates.sort();
    if candidates.is_empty() {
        if required {
            return Err(format!("required glob artifact has no matches: {path_text}"));
        }
        warnings.push(format!("optional glob artifact has no matches: {path_text}"));
        return Ok((ArtifactStatus::Missing, Vec::new()));
    }

    let mut normalized = HashSet::<PathBuf>::new();
    let mut out = Vec::new();
    for candidate in candidates {
        if !candidate.exists() {
            continue;
        }
        let abs = if candidate.is_absolute() {
            candidate
        } else {
            project_root.join(&candidate)
        };

        if !allow_outside_project && !is_within_root(&abs, project_root) {
            return Err(format!(
                "artifact path '{}' is outside current working directory",
                path_text
            ));
        }
        if should_skip_default_excluded(path_text, &abs, project_root) {
            continue;
        }

        if normalized.insert(abs.clone()) {
            out.push(abs.to_string_lossy().to_string());
        }
    }

    if out.is_empty() {
        if required {
            return Err(format!("required glob artifact has no matches: {path_text}"));
        }
        warnings.push(format!("optional glob artifact has no matches after filtering: {path_text}"));
        return Ok((ArtifactStatus::Missing, Vec::new()));
    }

    let _ = fs::metadata(source).map_err(|err| format!("cannot access glob source '{}': {err}", path_text))?;
    Ok((ArtifactStatus::Included, out))
}

fn is_within_root(path: &Path, root: &Path) -> bool {
    path.starts_with(root)
}
