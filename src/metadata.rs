use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use crate::{collect::CollectedArtifact, gitinfo::GitInfo, manifest::Manifest};

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub schema_version: u32,
    pub run_id: String,
    pub run_name: String,
    pub project: String,
    pub tags: Vec<String>,
    pub timestamp: String,
    pub archive: ArchiveMeta,
    pub oss: OssMeta,
    pub artifacts: Vec<ArtifactMeta>,
    pub git: GitMeta,
    pub upload: UploadMeta,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArchiveMeta {
    pub filename: String,
    pub local_path: String,
    pub size_bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OssMeta {
    pub bucket: String,
    pub region: String,
    pub endpoint: String,
    pub remote_dir: String,
    pub archive_uri: String,
    pub metadata_uri: String,
    pub sha256_uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArtifactMeta {
    pub name: String,
    pub artifact_type: String,
    pub source_path: String,
    pub required: bool,
    pub status: String,
    pub matched_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitMeta {
    pub available: bool,
    pub repo_root: Option<String>,
    pub commit: Option<String>,
    pub branch: Option<String>,
    pub dirty: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadMeta {
    pub status: String,
    pub duration_seconds: Option<f64>,
    pub avg_mib_s: Option<f64>,
}

pub fn now_iso() -> String {
    let now: DateTime<Local> = Local::now();
    now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn build_metadata(
    manifest: &Manifest,
    run_id: &str,
    archive_filename: &str,
    archive_path: &str,
    size_bytes: u64,
    sha256: &str,
    remote_dir: &str,
    artifacts: &[CollectedArtifact],
    git_info: &GitInfo,
    status: &str,
    duration_seconds: Option<f64>,
    avg_mib_s: Option<f64>,
    warnings: &[String],
) -> Metadata {
    Metadata {
        schema_version: 1,
        run_id: run_id.to_string(),
        run_name: manifest.run.name.clone(),
        project: manifest.run.project.clone(),
        tags: manifest.run.tags.clone(),
        timestamp: now_iso(),
        archive: ArchiveMeta {
            filename: archive_filename.to_string(),
            local_path: archive_path.to_string(),
            size_bytes,
            sha256: sha256.to_string(),
        },
        oss: OssMeta {
            bucket: manifest.oss.bucket.clone(),
            region: manifest.oss.region.clone(),
            endpoint: manifest.oss.endpoint.clone(),
            remote_dir: remote_dir.to_string(),
            archive_uri: format!(
                "oss://{}/{}/{}",
                manifest.oss.bucket,
                remote_dir.trim_start_matches('/'),
                archive_filename
            ),
            metadata_uri: format!(
                "oss://{}/{}/{}.meta.json",
                manifest.oss.bucket,
                remote_dir.trim_start_matches('/'),
                run_id
            ),
            sha256_uri: format!(
                "oss://{}/{}/{}.sha256",
                manifest.oss.bucket,
                remote_dir.trim_start_matches('/'),
                run_id
            ),
        },
        artifacts: artifacts
            .iter()
            .map(|artifact| ArtifactMeta {
                name: artifact.name.clone(),
                artifact_type: artifact.artifact_type.clone(),
                source_path: artifact.source_path.clone(),
                required: artifact.required,
                status: format!("{}", artifact.status_string()),
                matched_count: artifact.matched_count(),
            })
            .collect(),
        git: GitMeta {
            available: git_info.available,
            repo_root: git_info.repo_root.clone(),
            commit: git_info.commit.clone(),
            branch: git_info.branch.clone(),
            dirty: git_info.dirty,
        },
        upload: UploadMeta {
            status: status.to_string(),
            duration_seconds,
            avg_mib_s,
        },
        warnings: warnings.to_vec(),
    }
}

impl CollectedArtifact {
    fn status_string(&self) -> &str {
        match self.status {
            crate::collect::ArtifactStatus::Included => "included",
            crate::collect::ArtifactStatus::Missing => "missing",
        }
    }
}
