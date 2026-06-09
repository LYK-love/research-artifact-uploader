from __future__ import annotations

from datetime import datetime
from typing import Any

from .collect import CollectedArtifact
from .manifest import Manifest
from .gitinfo import GitInfo


def now_iso() -> str:
    return datetime.now().astimezone().isoformat(timespec="seconds")


def _artifact_dict(artifacts: list[CollectedArtifact]) -> list[dict[str, Any]]:
    out = []
    for artifact in artifacts:
        out.append(
            {
                "name": artifact.name,
                "type": artifact.type,
                "source_path": artifact.source_path,
                "required": artifact.required,
                "status": artifact.status,
                "matched_count": artifact.matched_count,
            }
        )
    return out


def build_metadata(
    manifest: Manifest,
    run_id: str,
    archive_filename: str,
    archive_path: str,
    size_bytes: int,
    sha256: str,
    remote_dir: str,
    artifacts: list[CollectedArtifact],
    git_info,
    status: str,
    duration_seconds: float | None,
    avg_mib_s: float | None,
    warnings: list[str],
    timestamp: str | None = None,
) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "run_id": run_id,
        "run_name": manifest.run.name,
        "project": manifest.run.project,
        "tags": list(manifest.run.tags),
        "timestamp": timestamp or now_iso(),
        "archive": {
            "filename": archive_filename,
            "local_path": archive_path,
            "size_bytes": size_bytes,
            "sha256": sha256,
        },
        "oss": {
            "bucket": manifest.oss.bucket,
            "region": manifest.oss.region,
            "endpoint": manifest.oss.endpoint,
            "remote_dir": remote_dir,
            "archive_uri": f"oss://{manifest.oss.bucket}/{remote_dir}/{archive_filename}",
            "metadata_uri": f"oss://{manifest.oss.bucket}/{remote_dir}/{run_id}.meta.json",
            "sha256_uri": f"oss://{manifest.oss.bucket}/{remote_dir}/{run_id}.sha256",
        },
        "artifacts": _artifact_dict(artifacts),
        "git": {
            "available": git_info.available,
            "repo_root": git_info.repo_root,
            "commit": git_info.commit,
            "branch": git_info.branch,
            "dirty": git_info.dirty,
        },
        "upload": {
            "status": status,
            "duration_seconds": duration_seconds,
            "avg_mib_s": avg_mib_s,
        },
        "warnings": list(warnings),
    }
