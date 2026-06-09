from __future__ import annotations

import json
from datetime import datetime
from pathlib import Path

import typer
from rich.console import Console
from rich.table import Table

from .archive import (
    compute_sha256,
    create_archive,
    make_timestamp,
    run_id_for_run,
    write_git_snapshot,
    write_manifest_snapshot,
)
from .collect import CollectedArtifact, collect_artifacts
from .gitinfo import read_git_info
from .manifest import parse_manifest
from .metadata import build_metadata, now_iso
from .oss import check_access, upload_file
from .records import append_jsonl, append_markdown, read_records

app = typer.Typer(help="Upload research artifacts to OSS via ossutil.")
console = Console()


def _project_root() -> Path:
    return Path.cwd()


def _load_context(manifest_path: Path, allow_outside_project: bool):
    manifest = parse_manifest(manifest_path)
    artifacts, warnings = collect_artifacts(
        manifest,
        project_root=_project_root(),
        allow_outside_project=allow_outside_project,
    )
    return manifest, artifacts, warnings


def _remote_dir(manifest, run_id: str) -> str:
    return f"{manifest.oss.remote_dir.rstrip('/')}/{run_id.rsplit('_', 1)[1]}"


def _print_artifacts(artifacts: list[CollectedArtifact]) -> None:
    table = Table(title="Artifacts")
    table.add_column("name")
    table.add_column("type")
    table.add_column("required")
    table.add_column("status")
    table.add_column("paths")
    for artifact in artifacts:
        table.add_row(
            artifact.name,
            artifact.type,
            str(artifact.required),
            artifact.status,
            ", ".join(artifact.matched_paths) if artifact.matched_paths else "(none)",
        )
    console.print(table)


def _metadata_path(manifest, run_id: str) -> Path:
    return Path(manifest.archive.output_dir) / f"{run_id}.meta.json"


def _write_metadata_file(
    manifest,
    run_id: str,
    archive_path: Path,
    size_bytes: int,
    sha256_value: str,
    remote_dir: str,
    artifacts: list[CollectedArtifact],
    git_info,
    status: str,
    warnings: list[str],
    duration_seconds: float | None,
    avg_mib_s: float | None,
) -> Path:
    payload = build_metadata(
        manifest=manifest,
        run_id=run_id,
        archive_filename=archive_path.name,
        archive_path=str(archive_path),
        size_bytes=size_bytes,
        sha256=sha256_value,
        remote_dir=remote_dir,
        artifacts=artifacts,
        git_info=git_info,
        status=status,
        duration_seconds=duration_seconds,
        avg_mib_s=avg_mib_s,
        warnings=warnings,
        timestamp=now_iso(),
    )
    path = _metadata_path(manifest, run_id)
    path.write_text(json.dumps(payload, indent=2, ensure_ascii=False), encoding="utf-8")
    return path


@app.command()
def check(
    manifest: Path = typer.Option(..., "--manifest", "-m", exists=True),
    allow_outside_project: bool = typer.Option(
        False, "--allow-outside-project", help="Allow artifact paths outside cwd."
    ),
):
    try:
        manifest_data, artifacts, warnings = _load_context(manifest, allow_outside_project)
        Path(manifest_data.archive.output_dir).mkdir(parents=True, exist_ok=True)

        console.print("manifest: parsed")
        _print_artifacts(artifacts)
        for item in warnings:
            console.print(f"warning: {item}")

        if not manifest_data.oss.bucket:
            raise ValueError("oss.bucket is required")
        if not manifest_data.oss.region:
            raise ValueError("oss.region is required")
        if not manifest_data.oss.endpoint:
            raise ValueError("oss.endpoint is required")

        ok, msg = check_access(
            manifest_data.oss.bucket,
            manifest_data.oss.endpoint,
            manifest_data.oss.region,
        )
        if ok:
            console.print("ossutil ls check passed")
        else:
            console.print(f"warning: ossutil ls check failed: {msg}")

        console.print("check complete")
    except Exception as exc:
        console.print(f"check failed: {exc}")
        raise typer.Exit(code=1)


@app.command()
def pack(
    manifest: Path = typer.Option(..., "--manifest", "-m", exists=True),
    allow_outside_project: bool = typer.Option(
        False,
        "--allow-outside-project",
        help="Allow artifact paths outside cwd.",
    ),
):
    try:
        manifest_data, artifacts, warnings = _load_context(manifest, allow_outside_project)

        run_id = run_id_for_run(manifest_data.run.name, make_timestamp())
        out_dir = Path(manifest_data.archive.output_dir)
        out_dir.mkdir(parents=True, exist_ok=True)

        run_name = manifest_data.run.name
        manifest_snap = out_dir / f"{run_id}.manifest.yaml"
        git_snap = out_dir / f"{run_id}.git_info.json"
        archive_path = out_dir / f"{run_id}.tar.gz"
        metadata_path = _metadata_path(manifest_data, run_id)
        sha_path = out_dir / f"{run_id}.sha256"

        write_manifest_snapshot(manifest_data, manifest_snap)
        git_info = read_git_info()
        write_git_snapshot(
            {
                "available": git_info.available,
                "repo_root": git_info.repo_root,
                "commit": git_info.commit,
                "branch": git_info.branch,
                "dirty": git_info.dirty,
            },
            git_snap,
        )

        if manifest_data.archive.include_metadata:
            metadata_path.write_text("{}", encoding="utf-8")

        create_archive(
            manifest=manifest_data,
            artifacts=artifacts,
            run_id=run_id,
            run_name=run_name,
            output_dir=out_dir,
            manifest_yaml_path=manifest_snap,
            git_info_path=git_snap,
            metadata_path=metadata_path if manifest_data.archive.include_metadata else None,
        )

        sha256_value, size_bytes = compute_sha256(archive_path)
        sha_path.write_text(f"{sha256_value}  {archive_path.name}\\n", encoding="utf-8")

        remote_dir = _remote_dir(manifest_data, run_id)
        _write_metadata_file(
            manifest_data,
            run_id=run_id,
            archive_path=archive_path,
            size_bytes=size_bytes,
            sha256_value=sha256_value,
            remote_dir=remote_dir,
            artifacts=artifacts,
            git_info=git_info,
            status="not_uploaded",
            warnings=warnings,
            duration_seconds=None,
            avg_mib_s=None,
        )

        for item in warnings:
            console.print(f"warning: {item}")
        console.print(f"archive: {archive_path}")
        console.print(f"metadata: {_metadata_path(manifest_data, run_id)}")
        console.print(f"sha256: {sha_path}")
    except Exception as exc:
        console.print(f"pack failed: {exc}")
        raise typer.Exit(code=1)


@app.command()
def upload(
    manifest: Path = typer.Option(..., "--manifest", "-m", exists=True),
    dry_run: bool = typer.Option(False, "--dry-run"),
    no_upload: bool = typer.Option(False, "--no-upload"),
    no_record: bool = typer.Option(False, "--no-record"),
    allow_outside_project: bool = typer.Option(
        False, "--allow-outside-project", help="Allow artifact paths outside cwd."
    ),
):
    try:
        manifest_data, artifacts, warnings = _load_context(manifest, allow_outside_project)

        run_id = run_id_for_run(manifest_data.run.name, make_timestamp())
        out_dir = Path(manifest_data.archive.output_dir)
        run_name = manifest_data.run.name
        remote_dir = _remote_dir(manifest_data, run_id)

        if dry_run:
            _print_artifacts(artifacts)
            console.print(f"planned run_id: {run_id}")
            console.print(f"planned remote dir: oss://{manifest_data.oss.bucket}/{remote_dir}")
            return

        out_dir.mkdir(parents=True, exist_ok=True)
        manifest_snap = out_dir / f"{run_id}.manifest.yaml"
        git_snap = out_dir / f"{run_id}.git_info.json"
        archive_path = out_dir / f"{run_id}.tar.gz"
        metadata_path = _metadata_path(manifest_data, run_id)
        sha_path = out_dir / f"{run_id}.sha256"

        git_info = read_git_info()
        write_manifest_snapshot(manifest_data, manifest_snap)
        write_git_snapshot(
            {
                "available": git_info.available,
                "repo_root": git_info.repo_root,
                "commit": git_info.commit,
                "branch": git_info.branch,
                "dirty": git_info.dirty,
            },
            git_snap,
        )

        if manifest_data.archive.include_metadata:
            metadata_path.write_text("{}", encoding="utf-8")

        create_archive(
            manifest=manifest_data,
            artifacts=artifacts,
            run_id=run_id,
            run_name=run_name,
            output_dir=out_dir,
            manifest_yaml_path=manifest_snap,
            git_info_path=git_snap,
            metadata_path=metadata_path if manifest_data.archive.include_metadata else None,
        )

        sha256_value, size_bytes = compute_sha256(archive_path)
        sha_path.write_text(f"{sha256_value}  {archive_path.name}\\n", encoding="utf-8")

        if no_upload:
            duration_seconds = 0.0
            avg_mib_s = None
            status = "not_uploaded"
            for item in warnings:
                console.print(f"warning: {item}")
        else:
            upload_started_at = datetime.now()
            status = "pending"

            for item in warnings:
                console.print(f"warning: {item}")

            try:
                _write_metadata_file(
                    manifest_data,
                    run_id=run_id,
                    archive_path=archive_path,
                    size_bytes=size_bytes,
                    sha256_value=sha256_value,
                    remote_dir=remote_dir,
                    artifacts=artifacts,
                    git_info=git_info,
                    status=status,
                    warnings=warnings,
                    duration_seconds=None,
                    avg_mib_s=None,
                )

                archive_result = upload_file(
                    archive_path,
                    manifest_data.oss.bucket,
                    remote_dir,
                    archive_path.name,
                    manifest_data.oss.endpoint,
                    manifest_data.oss.region,
                )

                _write_metadata_file(
                    manifest_data,
                    run_id=run_id,
                    archive_path=archive_path,
                    size_bytes=size_bytes,
                    sha256_value=sha256_value,
                    remote_dir=remote_dir,
                    artifacts=artifacts,
                    git_info=git_info,
                    status="archive_uploaded",
                    warnings=warnings,
                    duration_seconds=None,
                    avg_mib_s=None,
                )

                upload_file(
                    metadata_path,
                    manifest_data.oss.bucket,
                    remote_dir,
                    metadata_path.name,
                    manifest_data.oss.endpoint,
                    manifest_data.oss.region,
                )

                upload_file(
                    sha_path,
                    manifest_data.oss.bucket,
                    remote_dir,
                    sha_path.name,
                    manifest_data.oss.endpoint,
                    manifest_data.oss.region,
                )

                duration_seconds = (datetime.now() - upload_started_at).total_seconds()
                avg_mib_s = archive_result.avg_mib_s
                if avg_mib_s is None and duration_seconds > 0:
                    avg_mib_s = size_bytes / max(duration_seconds, 1e-6) / (1024 ** 2)
                status = "success"

                if not no_record:
                    summary = {
                        "time": now_iso(),
                        "run_id": run_id,
                        "run_name": manifest_data.run.name,
                        "project": manifest_data.run.project,
                        "archive_uri": f"oss://{manifest_data.oss.bucket}/{remote_dir}/{archive_path.name}",
                        "metadata_uri": f"oss://{manifest_data.oss.bucket}/{remote_dir}/{run_id}.meta.json",
                        "sha256": sha256_value,
                        "size_bytes": size_bytes,
                        "avg_mib_s": avg_mib_s,
                        "status": status,
                    }
                    append_jsonl(Path(manifest_data.records.jsonl), summary)
                    append_markdown(Path(manifest_data.records.markdown), summary)
            except Exception:
                duration_seconds = None
                avg_mib_s = None
                status = "failed"
                _write_metadata_file(
                    manifest_data,
                    run_id=run_id,
                    archive_path=archive_path,
                    size_bytes=size_bytes,
                    sha256_value=sha256_value,
                    remote_dir=remote_dir,
                    artifacts=artifacts,
                    git_info=git_info,
                    status=status,
                    warnings=warnings + ["upload flow failed, inspect previous output and rerun command"],
                    duration_seconds=None,
                    avg_mib_s=None,
                )
                raise

        _write_metadata_file(
            manifest_data,
            run_id=run_id,
            archive_path=archive_path,
            size_bytes=size_bytes,
            sha256_value=sha256_value,
            remote_dir=remote_dir,
            artifacts=artifacts,
            git_info=git_info,
            status=status,
            warnings=warnings,
            duration_seconds=duration_seconds,
            avg_mib_s=avg_mib_s,
        )

        console.print(f"archive: {archive_path}")
        console.print(f"metadata: {metadata_path}")
        console.print(f"sha256: {sha_path}")
        console.print(f"duration: {duration_seconds:.2f}s")
        if avg_mib_s is None:
            console.print("avg MiB/s: n/a")
        else:
            console.print(f"avg MiB/s: {avg_mib_s:.2f}")
        console.print(f"remote uri: oss://{manifest_data.oss.bucket}/{remote_dir}/{archive_path.name}")

        for item in warnings:
            console.print(f"warning: {item}")
    except Exception as exc:
        console.print(f"upload failed: {exc}")
        raise typer.Exit(code=1)


@app.command()
def records(
    jsonl: Path = typer.Option("docs/upload_records.jsonl", "--jsonl", help="Path to JSONL record file."),
    last: int = typer.Option(10, "--last", min=1, help="Rows to print"),
):
    rows = read_records(jsonl, last=last)
    if not rows:
        console.print("No records found.")
        return

    table = Table(title="Upload Records")
    table.add_column("Time")
    table.add_column("Run")
    table.add_column("Project")
    table.add_column("Size")
    table.add_column("Speed")
    table.add_column("Remote URI")
    table.add_column("SHA256")

    for row in rows:
        size = row.get("size_bytes", 0)
        speed = row.get("avg_mib_s")
        table.add_row(
            row.get("time", ""),
            row.get("run_name", ""),
            row.get("project", ""),
            f"{size / (1024 ** 2):.1f} MiB",
            "n/a" if speed is None else f"{speed:.2f} MiB/s",
            row.get("archive_uri", ""),
            str(row.get("sha256", ""))[:12],
        )

    console.print(table)


if __name__ == "__main__":
    app()
