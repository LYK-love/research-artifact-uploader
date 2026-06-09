from pathlib import Path
import tarfile
import hashlib

from research_artifact_uploader.manifest import parse_manifest
from research_artifact_uploader.collect import collect_artifacts
from research_artifact_uploader.archive import create_archive, compute_sha256, run_id_for_run


def test_archive_has_expected_internal_paths(tmp_path: Path) -> None:
    manifest_path = tmp_path / "artifacts.yaml"
    manifest_path.write_text(
        """
run:
  name: demo
  project: proj
artifacts:
  - name: ckpt
    path: run/ckpt
    type: directory
    required: true
  - name: metrics
    path: run/metrics.jsonl
    type: file
    required: true
archive:
  output_dir: .rau/archives
"""
    )
    (tmp_path / "run/ckpt").mkdir(parents=True)
    (tmp_path / "run/ckpt/model.bin").write_text("model")
    (tmp_path / "run/metrics.jsonl").write_text("{}\\n")

    parsed = parse_manifest(manifest_path)
    artifacts, _ = collect_artifacts(parsed, project_root=tmp_path)

    out_dir = tmp_path / ".rau/archives"
    run_id = run_id_for_run("demo")
    archive = create_archive(
        manifest=parsed,
        artifacts=artifacts,
        run_id=run_id,
        run_name="demo",
        output_dir=out_dir,
        manifest_yaml_path=None,
        git_info_path=None,
        metadata_path=None,
    )
    with tarfile.open(archive, "r:gz") as t:
        names = t.getnames()
    assert f"demo/artifacts/ckpt/model.bin" in names
    assert f"demo/artifacts/metrics/metrics.jsonl" in names


def test_sha256_calculation(tmp_path: Path) -> None:
    p = tmp_path / "x.bin"
    p.write_bytes(b"abc")
    h = hashlib.sha256(b"abc").hexdigest()
    got, size = compute_sha256(p)
    assert size == 3
    assert got == h
