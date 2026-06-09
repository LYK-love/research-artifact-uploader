use std::{
    collections::HashSet,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use flate2::{write::GzEncoder, Compression};
use sha2::{Digest, Sha256};
use tar::Builder;
use walkdir::WalkDir;

use crate::{
    collect::{CollectedArtifact, ArtifactStatus},
    manifest::Manifest,
    paths::DEFAULT_EXCLUDED_DIRS,
    AppResult,
};

pub fn make_timestamp() -> String {
    chrono::Local::now().format("%Y%m%d_%H%M%S").to_string()
}

pub fn run_id_for_run(run_name: &str, timestamp: Option<&str>) -> String {
    match timestamp {
        Some(v) => format!("{run_name}_{v}"),
        None => format!("{run_name}_{}", make_timestamp()),
    }
}

pub fn compute_sha256(path: &Path) -> AppResult<(String, u64)> {
    let mut file = File::open(path).map_err(|e| format!("open archive failed: {e}"))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 1024 * 1024];
    let mut total = 0u64;

    loop {
        let n = file.read(&mut buf).map_err(|e| format!("read archive failed: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        total += n as u64;
    }

    Ok((format!("{:x}", hasher.finalize()), total))
}

pub fn write_sha256(archive: &Path, sha256: &str) -> AppResult<PathBuf> {
    let sha_path = archive.with_extension("sha256");
    let mut fh = File::create(&sha_path).map_err(|e| format!("write sha256 failed: {e}"))?;
    writeln!(
        fh,
        "{}  {}",
        sha256,
        archive.file_name().and_then(|n| n.to_str()).unwrap_or("archive")
    )
    .map_err(|e| format!("write sha256 failed: {e}"))?;
    Ok(sha_path)
}

pub fn create_archive(
    manifest: &Manifest,
    artifacts: &[CollectedArtifact],
    run_id: &str,
    run_name: &str,
    output_dir: &Path,
    project_root: &Path,
    manifest_yaml_path: Option<&Path>,
    git_info_path: Option<&Path>,
    metadata_path: Option<&Path>,
) -> AppResult<PathBuf> {
    fs::create_dir_all(output_dir).map_err(|e| format!("create output dir failed: {e}"))?;
    let archive_path = output_dir.join(format!("{run_id}.tar.gz"));

    let output = File::create(&archive_path).map_err(|e| format!("create archive failed: {e}"))?;
    let encoder = GzEncoder::new(output, Compression::new(manifest.archive.compression_level));
    let mut tar = Builder::new(encoder);

    let mut used: HashSet<String> = HashSet::new();
    for artifact in artifacts.iter().filter(|x| x.status == ArtifactStatus::Included) {
        for (source, dest_name) in
            iter_artifact_members(artifact, run_name, project_root, &mut used)?
        {
            tar.append_path_with_name(&source, &dest_name)
                .map_err(|e| format!("add file to archive failed: {e}"))?;
        }
    }

    if manifest.archive.include_manifest {
        if let Some(manifest_path) = manifest_yaml_path {
            tar.append_path_with_name(manifest_path, format!("{run_name}/manifest.yaml"))
                .map_err(|e| format!("append manifest failed: {e}"))?;
        }
    }
    if manifest.archive.include_git_info {
        if let Some(git_path) = git_info_path {
            tar.append_path_with_name(git_path, format!("{run_name}/git_info.json"))
                .map_err(|e| format!("append git_info failed: {e}"))?;
        }
    }
    if manifest.archive.include_metadata {
        if let Some(meta_path) = metadata_path {
            tar.append_path_with_name(meta_path, format!("{run_name}/metadata.json"))
                .map_err(|e| format!("append metadata failed: {e}"))?;
        }
    }

    let encoder = tar
        .into_inner()
        .map_err(|e| format!("close tar builder failed: {e}"))?;
    encoder
        .finish()
        .map_err(|e| format!("close gzip encoder failed: {e}"))?;

    Ok(archive_path)
}

fn iter_artifact_members(
    artifact: &CollectedArtifact,
    run_name: &str,
    project_root: &Path,
    used: &mut HashSet<String>,
) -> AppResult<Vec<(PathBuf, String)>> {
    let mut out = Vec::new();

    match artifact.artifact_type.as_str() {
        "file" => {
            if let Some(first) = artifact.matched_paths.first() {
                let source = PathBuf::from(first);
                let base = source.file_name().and_then(|v| v.to_str()).unwrap_or("file");
                let final_name = unique_name(base, used);
                let dest = Path::new(run_name)
                    .join("artifacts")
                    .join(&artifact.name)
                    .join(final_name);
                out.push((source, dest.to_string_lossy().to_string()));
            }
        }
        "directory" => {
            if let Some(root_path) = artifact.matched_paths.first() {
                let source = PathBuf::from(root_path);
                for entry in WalkDir::new(&source).into_iter().filter_map(Result::ok) {
                    if entry.file_type().is_dir() {
                        continue;
                    }
                    let path = entry.path().to_path_buf();
                    if should_skip_file(project_root, &source, &path) {
                        continue;
                    }
                    let rel = path.strip_prefix(&source).unwrap_or(&path);
                    let dest = Path::new(run_name)
                        .join("artifacts")
                        .join(&artifact.name)
                        .join(rel);
                    out.push((path, dest.to_string_lossy().to_string()));
                }
            }
        }
        "glob" => {
            for path_text in &artifact.matched_paths {
                let p = PathBuf::from(path_text);
                if p.is_file() {
                    let base = p.file_name().and_then(|v| v.to_str()).unwrap_or("file");
                    let final_name = unique_name(base, used);
                    let dest = Path::new(run_name)
                        .join("artifacts")
                        .join(&artifact.name)
                        .join(final_name);
                    out.push((p, dest.to_string_lossy().to_string()));
                } else if p.is_dir() {
                    let group = unique_name(
                        p.file_name().and_then(|v| v.to_str()).unwrap_or("dir"),
                        used,
                    );
                    for entry in WalkDir::new(&p).into_iter().filter_map(Result::ok) {
                        if entry.file_type().is_dir() {
                            continue;
                        }
                        let path = entry.path().to_path_buf();
                        if should_skip_file(project_root, &p, &path) {
                            continue;
                        }
                        let rel = path.strip_prefix(&p).unwrap_or(&path);
                        let dest = Path::new(run_name)
                            .join("artifacts")
                            .join(&artifact.name)
                            .join(&group)
                            .join(rel);
                        out.push((path, dest.to_string_lossy().to_string()));
                    }
                }
            }
        }
        _ => {}
    }

    Ok(out)
}

fn should_skip_file(project_root: &Path, root: &Path, path: &Path) -> bool {
    let rel_root = match root.strip_prefix(project_root) {
        Ok(r) => r,
        Err(_) => return false,
    };
    let rel = match path.strip_prefix(project_root) {
        Ok(r) => r,
        Err(_) => return false,
    };
    if rel_root
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .any(|part| DEFAULT_EXCLUDED_DIRS.iter().any(|d| part == *d))
    {
        return true;
    }
    for part in rel.components() {
        let part = part.as_os_str().to_string_lossy();
        if DEFAULT_EXCLUDED_DIRS.iter().any(|d| part == *d) {
            return true;
        }
    }
    false
}

fn unique_name(base: &str, used: &mut HashSet<String>) -> String {
    if !used.contains(base) {
        used.insert(base.to_string());
        return base.to_string();
    }

    let stem = Path::new(base)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(base);
    let suffix = Path::new(base).extension().and_then(|s| s.to_str()).unwrap_or("");

    let mut idx = 1u32;
    loop {
        let candidate = if suffix.is_empty() {
            format!("{stem}_{idx}")
        } else {
            format!("{stem}_{idx}.{suffix}")
        };
        if !used.contains(&candidate) {
            used.insert(candidate.clone());
            return candidate;
        }
        idx += 1;
    }
}

pub fn write_manifest_snapshot(content: &str, path: &Path) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create manifest parent failed: {e}"))?;
    }
    fs::write(path, content).map_err(|e| format!("write manifest snapshot failed: {e}"))
}
