use std::{fs, path::Path};

use serde::{Deserialize, Serialize};
use serde_yaml::Value;

pub const DEFAULT_ARCHIVE_OUTPUT_DIR: &str = ".rau/archives";
pub const DEFAULT_ARCHIVE_FORMAT: &str = "tar.gz";
pub const DEFAULT_ARCHIVE_COMPRESSION_LEVEL: u32 = 3;
pub const DEFAULT_RECORDS_JSONL: &str = "docs/upload_records.jsonl";
pub const DEFAULT_RECORDS_MARKDOWN: &str = "docs/upload_records.md";
pub const DEFAULT_OSS_BUCKET: &str = "luyukuan-research";
pub const DEFAULT_OSS_REGION: &str = "cn-shanghai";
pub const DEFAULT_OSS_ENDPOINT: &str = "oss-accelerate.aliyuncs.com";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub run: RunConfig,
    #[serde(default)]
    pub artifacts: Vec<ArtifactConfig>,
    #[serde(default)]
    pub archive: ArchiveConfig,
    #[serde(default)]
    pub oss: OssConfig,
    #[serde(default)]
    pub records: RecordsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfig {
    pub name: String,
    pub project: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactConfig {
    pub name: String,
    pub path: String,
    #[serde(rename = "type")]
    pub artifact_type: String,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveConfig {
    #[serde(default = "default_archive_output_dir")]
    pub output_dir: String,
    #[serde(default = "default_archive_format")]
    pub format: String,
    #[serde(default = "default_archive_compression_level")]
    pub compression_level: u32,
    #[serde(default = "default_true")]
    pub include_manifest: bool,
    #[serde(default = "default_true")]
    pub include_git_info: bool,
    #[serde(default = "default_true")]
    pub include_metadata: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OssConfig {
    #[serde(default = "default_bucket")]
    pub bucket: String,
    #[serde(default = "default_region")]
    pub region: String,
    #[serde(default = "default_endpoint")]
    pub endpoint: String,
    pub remote_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordsConfig {
    #[serde(default = "default_records_jsonl")]
    pub jsonl: String,
    #[serde(default = "default_records_markdown")]
    pub markdown: String,
}

fn default_true() -> bool {
    true
}

fn default_archive_output_dir() -> String {
    DEFAULT_ARCHIVE_OUTPUT_DIR.to_string()
}
fn default_archive_format() -> String {
    DEFAULT_ARCHIVE_FORMAT.to_string()
}
fn default_archive_compression_level() -> u32 {
    DEFAULT_ARCHIVE_COMPRESSION_LEVEL
}
fn default_records_jsonl() -> String {
    DEFAULT_RECORDS_JSONL.to_string()
}
fn default_records_markdown() -> String {
    DEFAULT_RECORDS_MARKDOWN.to_string()
}
fn default_bucket() -> String {
    DEFAULT_OSS_BUCKET.to_string()
}
fn default_region() -> String {
    DEFAULT_OSS_REGION.to_string()
}
fn default_endpoint() -> String {
    DEFAULT_OSS_ENDPOINT.to_string()
}

impl Default for ArchiveConfig {
    fn default() -> Self {
        Self {
            output_dir: default_archive_output_dir(),
            format: default_archive_format(),
            compression_level: default_archive_compression_level(),
            include_manifest: true,
            include_git_info: true,
            include_metadata: true,
        }
    }
}

impl Default for RecordsConfig {
    fn default() -> Self {
        Self {
            jsonl: default_records_jsonl(),
            markdown: default_records_markdown(),
        }
    }
}

impl Default for OssConfig {
    fn default() -> Self {
        Self {
            bucket: default_bucket(),
            region: default_region(),
            endpoint: default_endpoint(),
            remote_dir: String::new(),
        }
    }
}

pub fn parse_manifest(path: &Path) -> Result<Manifest, String> {
    let raw = fs::read_to_string(path).map_err(|err| format!("cannot read manifest: {err}"))?;
    let payload: Value = serde_yaml::from_str(&raw).map_err(|err| format!("invalid manifest yaml: {err}"))?;

    let mapping = payload
        .as_mapping()
        .ok_or_else(|| "manifest root must be a mapping".to_string())?;

    let mut m: Manifest = serde_yaml::from_value(Value::Mapping(mapping.clone()))
        .map_err(|err| format!("invalid manifest schema: {err}"))?;

    if m.run.name.trim().is_empty() {
        return Err("run.name is required".to_string());
    }
    if m.run.project.trim().is_empty() {
        return Err("run.project is required".to_string());
    }

    if m.archive.format != "tar.gz" {
        return Err("only tar.gz archive format is supported".to_string());
    }
    if !(1..=9).contains(&m.archive.compression_level) {
        return Err("archive.compression_level must be in [1,9]".to_string());
    }

    if m.oss.remote_dir.trim().is_empty() {
        return Err("oss.remote_dir is required".to_string());
    }

    let allowed_types = ["file", "directory", "glob"];
    for (i, artifact) in m.artifacts.iter().enumerate() {
        if !allowed_types.contains(&artifact.artifact_type.as_str()) {
            return Err(format!(
                "artifacts[{i}].type must be file, directory, or glob"
            ));
        }
        if artifact.name.trim().is_empty() {
            return Err(format!("artifacts[{i}].name must not be empty"));
        }
        if artifact.path.trim().is_empty() {
            return Err(format!("artifacts[{i}].path must not be empty"));
        }
    }

    if m.records.jsonl.trim().is_empty() {
        m.records.jsonl = DEFAULT_RECORDS_JSONL.to_string();
    }
    if m.records.markdown.trim().is_empty() {
        m.records.markdown = DEFAULT_RECORDS_MARKDOWN.to_string();
    }

    Ok(m)
}

impl Manifest {
    pub fn to_map(&self) -> Result<String, String> {
        serde_yaml::to_string(self).map_err(|err| format!("serialize manifest: {err}"))
    }
}
