from __future__ import annotations

from datetime import datetime
from pathlib import Path
import hashlib
import tarfile
import json
import yaml
import os

from .collect import CollectedArtifact
from .manifest import Manifest
from .paths import DEFAULT_EXCLUDED_DIRS


def make_timestamp() -> str:
    return datetime.now().strftime("%Y%m%d_%H%M%S")


def run_id_for_run(run_name: str, timestamp: str | None = None) -> str:
    return f"{run_name}_{timestamp or make_timestamp()}"


def compute_sha256(path: Path) -> tuple[str, int]:
    h = hashlib.sha256()
    size = 0
    with path.open("rb") as f:
        while True:
            chunk = f.read(1024 * 1024)
            if not chunk:
                break
            h.update(chunk)
            size += len(chunk)
    return h.hexdigest(), size


def _unique_name(base_name: str, used: set[str]) -> str:
    if base_name not in used:
        used.add(base_name)
        return base_name

    stem = Path(base_name).stem
    suffix = Path(base_name).suffix
    i = 1
    while True:
        candidate = f"{stem}_{i}{suffix}"
        if candidate not in used:
            used.add(candidate)
            return candidate
        i += 1


def _iter_archive_members(
    artifact: CollectedArtifact,
    run_name: str,
    project_root: Path,
    used_dest_names: set[str],
) -> list[tuple[Path, str]]:
    entries: list[tuple[Path, str]] = []

    def _add_file(file_path: Path, arc_dir: Path) -> None:
        base = file_path.name
        final = _unique_name(base, used_dest_names)
        entries.append((file_path, str((arc_dir / final).as_posix())))

    if artifact.type == "file":
        if not artifact.matched_paths:
            return entries
        src = Path(artifact.matched_paths[0])
        arc_dir = Path(run_name) / "artifacts" / artifact.name
        _add_file(src, arc_dir)
        return entries

    if artifact.type == "directory":
        src = Path(artifact.matched_paths[0]) if artifact.matched_paths else None
        if src is None or not src.exists():
            return entries
        for child in sorted(src.rglob("*")):
            if child.is_dir():
                continue
            if any(p in child.as_posix().split(os.sep) for p in DEFAULT_EXCLUDED_DIRS):
                continue
            rel = child.relative_to(src)
            dst = Path(run_name) / "artifacts" / artifact.name / rel
            entries.append((child, str(dst.as_posix())))
        return entries

    if artifact.type == "glob":
        for match_str in artifact.matched_paths:
            match = Path(match_str)
            if match.is_file():
                arc_dir = Path(run_name) / "artifacts" / artifact.name
                base = match.name
                final = _unique_name(base, used_dest_names)
                entries.append((match, str((arc_dir / final).as_posix())))
            elif match.is_dir():
                group = _unique_name(match.name, used_dest_names)
                for child in sorted(match.rglob("*")):
                    if child.is_dir():
                        continue
                    if any(p in child.as_posix().split(os.sep) for p in DEFAULT_EXCLUDED_DIRS):
                        continue
                    rel = child.relative_to(match)
                    dst = Path(run_name) / "artifacts" / artifact.name / group / rel
                    entries.append((child, str(dst.as_posix())))
        return entries

    return entries


def create_archive(
    manifest: Manifest,
    artifacts: list[CollectedArtifact],
    run_id: str,
    run_name: str,
    output_dir: Path,
    manifest_yaml_path: Path | None,
    git_info_path: Path | None,
    metadata_path: Path | None,
) -> Path:
    output_dir.mkdir(parents=True, exist_ok=True)
    archive_path = output_dir / f"{run_id}.tar.gz"

    used_dest_names: set[str] = set()
    with tarfile.open(archive_path, "w:gz", compresslevel=manifest.archive.compression_level) as tar:
        for artifact in artifacts:
            if artifact.status != "included":
                continue
            for source, arc_name in _iter_archive_members(artifact, run_name, Path.cwd(), used_dest_names):
                if source.exists() and source.is_file():
                    tar.add(source, arcname=arc_name, recursive=False)

        if manifest.archive.include_manifest and manifest_yaml_path is not None:
            tar.add(manifest_yaml_path, arcname=f"{run_name}/manifest.yaml", recursive=False)
        if manifest.archive.include_git_info and git_info_path is not None:
            tar.add(git_info_path, arcname=f"{run_name}/git_info.json", recursive=False)
        if manifest.archive.include_metadata and metadata_path is not None and metadata_path.exists():
            tar.add(metadata_path, arcname=f"{run_name}/metadata.json", recursive=False)

    return archive_path


def write_manifest_snapshot(manifest: Manifest, path: Path) -> None:
    path.write_text(yaml.safe_dump(manifest.to_dict(), sort_keys=False), encoding="utf-8")


def write_git_snapshot(payload: dict, path: Path) -> None:
    path.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")
