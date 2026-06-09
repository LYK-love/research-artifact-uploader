from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
import glob

from .manifest import ArtifactConfig, Manifest
from .paths import validate_artifact_name, should_skip_default_excluded


@dataclass
class CollectedArtifact:
    name: str
    type: str
    source_path: str
    required: bool
    status: str
    matched_paths: list[str] = field(default_factory=list)

    @property
    def matched_count(self) -> int:
        return len(self.matched_paths)


def _normalize_match_list(paths: list[Path], root: Path, artifact: ArtifactConfig, allow_outside_project: bool) -> list[Path]:
    normalized: list[Path] = []
    for p in paths:
        pp = p if p.is_absolute() else (root / p).resolve()
        if not allow_outside_project and not pp.is_relative_to(root):
            raise ValueError(f"artifact path '{artifact.path}' resolves outside cwd; use --allow-outside-project")
        if should_skip_default_excluded(artifact.path, pp, root):
            continue
        normalized.append(pp)
    return normalized


def _glob_matches(pattern: str, root: Path) -> list[Path]:
    matches = sorted(glob.glob(str(pattern)))
    return [Path(x) for x in matches]


def collect_artifacts(
    manifest: Manifest,
    project_root: Path,
    allow_outside_project: bool = False,
) -> tuple[list[CollectedArtifact], list[str]]:
    collected: list[CollectedArtifact] = []
    warnings: list[str] = []

    for artifact in manifest.artifacts:
        validate_artifact_name(artifact.name)
        raw = Path(artifact.path)
        resolved = raw if raw.is_absolute() else (project_root / raw).resolve()

        if not allow_outside_project:
            if artifact.type != "glob" and not resolved.is_relative_to(project_root):
                raise ValueError(
                    f"artifact path '{artifact.path}' is outside current working directory"
                )

        if artifact.type == "file":
            if not resolved.exists():
                if artifact.required:
                    raise FileNotFoundError(f"required file artifact missing: {artifact.path}")
                warnings.append(f"optional file artifact missing: {artifact.path}")
                status = "missing"
                matched: list[Path] = []
            elif not resolved.is_file():
                raise ValueError(f"file artifact must point to a regular file: {artifact.path}")
            else:
                status = "included"
                matched = [resolved]

        elif artifact.type == "directory":
            if not resolved.exists():
                if artifact.required:
                    raise FileNotFoundError(f"required directory artifact missing: {artifact.path}")
                warnings.append(f"optional directory artifact missing: {artifact.path}")
                status = "missing"
                matched = []
            elif not resolved.is_dir():
                raise ValueError(f"directory artifact must point to a directory: {artifact.path}")
            else:
                status = "included"
                matched = [resolved]

        elif artifact.type == "glob":
            paths = _glob_matches(artifact.path, project_root)
            if not paths:
                if artifact.required:
                    raise FileNotFoundError(f"required glob artifact has no matches: {artifact.path}")
                warnings.append(f"optional glob artifact has no matches: {artifact.path}")
                status = "missing"
                matched = []
            else:
                matched = _normalize_match_list(paths, project_root, artifact, allow_outside_project)
                status = "included"

        else:
            raise ValueError(f"unsupported artifact type: {artifact.type}")

        collected.append(
            CollectedArtifact(
                name=artifact.name,
                type=artifact.type,
                source_path=artifact.path,
                required=artifact.required,
                status=status,
                matched_paths=[str(p) for p in matched],
            )
        )

    return collected, warnings
