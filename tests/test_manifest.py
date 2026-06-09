from pathlib import Path

import pytest

from research_artifact_uploader.manifest import parse_manifest


def test_parse_manifest_defaults_and_required_path(tmp_path: Path) -> None:
    manifest = tmp_path / "artifacts.yaml"
    manifest.write_text(
        """
run:
  name: demo
  project: proj
artifacts:
  - name: metrics
    path: metrics.jsonl
    type: file
    required: true
"""
    )

    parsed = parse_manifest(manifest)
    assert parsed.run.name == "demo"
    assert parsed.archive.output_dir == ".rau/archives"


def test_parse_manifest_missing_run_name(tmp_path: Path) -> None:
    manifest = tmp_path / "artifacts.yaml"
    manifest.write_text(
        """
run:
  project: proj
artifacts: []
"""
    )
    with pytest.raises(ValueError, match="run.name"):
        parse_manifest(manifest)
