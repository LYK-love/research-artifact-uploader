from __future__ import annotations

from pathlib import Path
import json


def _write_parent(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)


def append_jsonl(records_path: Path, payload: dict) -> None:
    _write_parent(records_path)
    with records_path.open("a", encoding="utf-8") as f:
        f.write(json.dumps(payload, ensure_ascii=False) + "\\n")


def _human_size(size_bytes: int) -> str:
    gib = size_bytes / (1024 ** 3)
    if gib >= 1:
        return f"{gib:.1f} GiB"
    mib = size_bytes / (1024 ** 2)
    return f"{mib:.1f} MiB"


def append_markdown(records_path: Path, payload: dict) -> None:
    _write_parent(records_path)
    exists = records_path.exists()
    with records_path.open("a", encoding="utf-8") as f:
        if not exists:
            f.write("# Upload Records\\n\\n")
            f.write("| Time | Run | Project | Size | Speed | Remote URI | SHA256 |\\n")
            f.write("| --- | --- | --- | ---: | ---: | --- | --- |\\n")
        speed = payload.get("avg_mib_s")
        speed_display = "n/a" if speed is None else f"{speed:.2f}"
        f.write(
            f"| {payload['time']} | {payload['run_name']} | {payload['project']} | "
            f"{_human_size(payload['size_bytes'])} | {speed_display} MiB/s | "
            f"`{payload['archive_uri']}` | `{payload['sha256']}` |\\n"
        )


def read_records(records_path: Path, last: int = 10):
    if not records_path.exists():
        return []
    lines = records_path.read_text(encoding="utf-8").splitlines()
    rows = []
    for line in lines[-last:]:
        if not line.strip():
            continue
        try:
            rows.append(json.loads(line))
        except json.JSONDecodeError:
            continue
    return rows
