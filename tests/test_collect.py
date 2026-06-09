from pathlib import Path

import pytest

from research_artifact_uploader.manifest import parse_manifest
from research_artifact_uploader.collect import collect_artifacts


def test_required_file_missing_fails(tmp_path: Path) -> None:
    manifest = tmp_path / "artifacts.yaml"
    manifest.write_text(
        """
run:
  name: demo
  project: proj
artifacts:
  - name: missing
    path: absent.bin
    type: file
    required: true
"""
    )
    parsed = parse_manifest(manifest)
    with pytest.raises(FileNotFoundError):
        collect_artifacts(parsed, project_root=tmp_path)


def test_optional_glob_missing_warns(tmp_path: Path) -> None:
    manifest = tmp_path / "artifacts.yaml"
    manifest.write_text(
        """
run:
  name: demo
  project: proj
artifacts:
  - name: vids
    path: videos/*.mp4
    type: glob
    required: false
"""
    )
    parsed = parse_manifest(manifest)
    items, warnings = collect_artifacts(parsed, project_root=tmp_path)
    assert items[0].status == "missing"
    assert warnings
