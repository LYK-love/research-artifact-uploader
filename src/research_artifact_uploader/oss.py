from __future__ import annotations

import re
import shutil
from dataclasses import dataclass
from pathlib import Path

from .subprocess_utils import run_capture, run_stream


@dataclass
class OssUploadResult:
    success: bool
    avg_mib_s: float | None
    stdout: str
    stderr: str
    command: list[str]


def _require_ossutil() -> None:
    if not shutil.which("ossutil"):
        raise FileNotFoundError("ossutil not found")


def _parse_avg_speed(text: str) -> float | None:
    patterns = [
        r"avg\s+([0-9]+(?:\.[0-9]+)?)\s*MiB/s",
        r"average\s+speed:\s*([0-9]+(?:\.[0-9]+)?)\s*MiB/s",
        r"avg\s+([0-9]+(?:\.[0-9]+)?)\s*MB/s",
    ]
    for p in patterns:
        m = re.search(p, text, flags=re.IGNORECASE)
        if m:
            return float(m.group(1))
    return None


def oss_uri(bucket: str, remote_dir: str, filename: str) -> str:
    return f"oss://{bucket}/{remote_dir.rstrip('/')}/{filename}"


def check_access(bucket: str, endpoint: str, region: str) -> tuple[bool, str]:
    _require_ossutil()
    cmd = ["ossutil", "ls", f"oss://{bucket}", "-e", endpoint, "--region", region]
    result = run_capture(cmd)
    if result.returncode == 0:
        return True, result.stdout
    return False, (result.stdout or result.stderr or "").strip()


def upload_file(
    local_path: Path,
    bucket: str,
    remote_dir: str,
    filename: str,
    endpoint: str,
    region: str,
) -> OssUploadResult:
    _require_ossutil()
    uri = oss_uri(bucket, remote_dir, filename)
    cmd = [
        "ossutil",
        "cp",
        str(local_path),
        uri,
        "-e",
        endpoint,
        "--region",
        region,
    ]
    result = run_stream(cmd, stream=True)
    if result.returncode != 0:
        command_str = " ".join(cmd)
        stdout_snip = (result.stdout or "").replace("\\n", "\\n")[:800]
        raise RuntimeError(
            "ossutil cp failed (exit code {}).\ncommand: {}\nerror: {}\nsuggestion: retry command manually after fixing the cause.".format(
                result.returncode,
                command_str,
                stdout_snip or "(no output)",
            )
        )
    return OssUploadResult(
        success=True,
        avg_mib_s=_parse_avg_speed(result.stdout),
        stdout=result.stdout,
        stderr=result.stderr,
        command=result.command,
    )
