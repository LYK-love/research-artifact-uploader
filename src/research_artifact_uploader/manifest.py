from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, List

import yaml

DEFAULT_ARCHIVE_OUTPUT_DIR = ".rau/archives"
DEFAULT_ARCHIVE_FORMAT = "tar.gz"
DEFAULT_ARCHIVE_COMPRESSION_LEVEL = 3
DEFAULT_RECORDS_JSONL = "docs/upload_records.jsonl"
DEFAULT_RECORDS_MARKDOWN = "docs/upload_records.md"

DEFAULT_OSS_BUCKET = "luyukuan-research"
DEFAULT_OSS_REGION = "cn-shanghai"
DEFAULT_OSS_ENDPOINT = "oss-accelerate.aliyuncs.com"


@dataclass
class RunConfig:
    name: str
    project: str
    tags: List[str]


@dataclass
class ArtifactConfig:
    name: str
    path: str
    type: str
    required: bool


@dataclass
class ArchiveConfig:
    output_dir: str
    format: str
    compression_level: int
    include_manifest: bool
    include_git_info: bool
    include_metadata: bool


@dataclass
class OssConfig:
    bucket: str
    region: str
    endpoint: str
    remote_dir: str | None


@dataclass
class RecordsConfig:
    jsonl: str
    markdown: str


@dataclass
class Manifest:
    run: RunConfig
    artifacts: List[ArtifactConfig]
    archive: ArchiveConfig
    oss: OssConfig
    records: RecordsConfig

    def to_dict(self) -> Dict[str, Any]:
        return {
            "run": {
                "name": self.run.name,
                "project": self.run.project,
                "tags": list(self.run.tags),
            },
            "artifacts": [
                {
                    "name": artifact.name,
                    "path": artifact.path,
                    "type": artifact.type,
                    "required": artifact.required,
                }
                for artifact in self.artifacts
            ],
            "archive": {
                "output_dir": self.archive.output_dir,
                "format": self.archive.format,
                "compression_level": self.archive.compression_level,
                "include_manifest": self.archive.include_manifest,
                "include_git_info": self.archive.include_git_info,
                "include_metadata": self.archive.include_metadata,
            },
            "oss": {
                "bucket": self.oss.bucket,
                "region": self.oss.region,
                "endpoint": self.oss.endpoint,
                "remote_dir": self.oss.remote_dir,
            },
            "records": {
                "jsonl": self.records.jsonl,
                "markdown": self.records.markdown,
            },
        }


def _expect_mapping(payload: Dict[str, Any], key: str):
    value = payload.get(key)
    if not isinstance(value, dict):
        raise ValueError(f"{key} must be a mapping")
    return value


def _expect_list(payload: Dict[str, Any], key: str):
    value = payload.get(key)
    if value is None:
        return []
    if not isinstance(value, list):
        raise ValueError(f"{key} must be a list")
    return value


def _expect_str(value: Any, key: str) -> str:
    if not isinstance(value, str):
        raise ValueError(f"{key} must be a string")
    return value


def _expect_bool(value: Any, key: str, default: bool) -> bool:
    if value is None:
        return default
    if isinstance(value, bool):
        return value
    raise ValueError(f"{key} must be true or false")


def _artifact_type(value: Any) -> str:
    if value not in {"file", "directory", "glob"}:
        raise ValueError("artifact type must be file, directory, or glob")
    return value


def parse_manifest(path: Path) -> Manifest:
    raw = path.read_text(encoding="utf-8")
    payload = yaml.safe_load(raw)
    if not isinstance(payload, dict):
        raise ValueError("manifest must be YAML mapping")

    run = _expect_mapping(payload, "run")
    name = _expect_str(run.get("name"), "run.name")
    project = _expect_str(run.get("project"), "run.project")
    tags_raw = run.get("tags", [])
    if not isinstance(tags_raw, list) or any(not isinstance(x, str) for x in tags_raw):
        raise ValueError("run.tags must be list of strings")

    run_cfg = RunConfig(name=name, project=project, tags=list(tags_raw))

    artifacts_raw = _expect_list(payload, "artifacts")
    artifacts: List[ArtifactConfig] = []
    for i, item in enumerate(artifacts_raw):
        if not isinstance(item, dict):
            raise ValueError(f"artifacts[{i}] must be mapping")
        artifacts.append(
            ArtifactConfig(
                name=_expect_str(item.get("name"), f"artifacts[{i}].name"),
                path=_expect_str(item.get("path"), f"artifacts[{i}].path"),
                type=_artifact_type(item.get("type")),
                required=_expect_bool(item.get("required"), f"artifacts[{i}].required", False),
            )
        )

    archive_cfg_raw = _expect_mapping(payload, "archive")
    archive_cfg = ArchiveConfig(
        output_dir=_expect_str(archive_cfg_raw.get("output_dir", DEFAULT_ARCHIVE_OUTPUT_DIR), "archive.output_dir"),
        format=_expect_str(archive_cfg_raw.get("format", DEFAULT_ARCHIVE_FORMAT), "archive.format"),
        compression_level=int(archive_cfg_raw.get("compression_level", DEFAULT_ARCHIVE_COMPRESSION_LEVEL)),
        include_manifest=_expect_bool(archive_cfg_raw.get("include_manifest"), "archive.include_manifest", True),
        include_git_info=_expect_bool(archive_cfg_raw.get("include_git_info"), "archive.include_git_info", True),
        include_metadata=_expect_bool(archive_cfg_raw.get("include_metadata"), "archive.include_metadata", True),
    )
    if archive_cfg.format != "tar.gz":
        raise ValueError("only tar.gz archive format is supported")
    if not 1 <= archive_cfg.compression_level <= 9:
        raise ValueError("archive.compression_level must be in [1,9]")

    oss_cfg_raw = _expect_mapping(payload, "oss")
    oss_cfg = OssConfig(
        bucket=_expect_str(oss_cfg_raw.get("bucket", DEFAULT_OSS_BUCKET), "oss.bucket"),
        region=_expect_str(oss_cfg_raw.get("region", DEFAULT_OSS_REGION), "oss.region"),
        endpoint=_expect_str(oss_cfg_raw.get("endpoint", DEFAULT_OSS_ENDPOINT), "oss.endpoint"),
        remote_dir=oss_cfg_raw.get("remote_dir"),
    )
    if not isinstance(oss_cfg.remote_dir, str) or not oss_cfg.remote_dir:
        raise ValueError("oss.remote_dir is required")

    records_cfg_raw = _expect_mapping(payload, "records")
    records_cfg = RecordsConfig(
        jsonl=_expect_str(records_cfg_raw.get("jsonl", DEFAULT_RECORDS_JSONL), "records.jsonl"),
        markdown=_expect_str(records_cfg_raw.get("markdown", DEFAULT_RECORDS_MARKDOWN), "records.markdown"),
    )

    return Manifest(
        run=run_cfg,
        artifacts=artifacts,
        archive=archive_cfg,
        oss=oss_cfg,
        records=records_cfg,
    )
