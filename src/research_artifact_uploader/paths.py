from __future__ import annotations

from pathlib import Path
import re

ARTIFACT_NAME_RE = re.compile(r"^[A-Za-z0-9_.-]+$")
DEFAULT_EXCLUDED_DIRS = {".git", ".rau", "__pycache__"}


def validate_artifact_name(name: str) -> None:
    if not ARTIFACT_NAME_RE.match(name):
        raise ValueError(
            f"artifact name '{name}' is invalid; allowed chars are A-Z a-z 0-9 _ . -"
        )


def is_within_root(path: Path, root: Path) -> bool:
    try:
        path.resolve().relative_to(root.resolve())
        return True
    except ValueError:
        return False


def _first_component(path_text: str) -> str:
    normalized = Path(path_text).as_posix().lstrip("./")
    if not normalized:
        return ""
    return normalized.split("/")[0]


def should_skip_default_excluded(path_text: str, candidate: Path, root: Path) -> bool:
    explicit_root = _first_component(path_text)
    if explicit_root in DEFAULT_EXCLUDED_DIRS:
        return False

    try:
        relative = candidate.relative_to(root)
    except ValueError:
        return False

    return any(part in DEFAULT_EXCLUDED_DIRS for part in relative.parts)
