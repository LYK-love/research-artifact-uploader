from __future__ import annotations

import subprocess
from dataclasses import dataclass
from pathlib import Path


@dataclass
class GitInfo:
    available: bool
    repo_root: str | None = None
    commit: str | None = None
    branch: str | None = None
    dirty: bool | None = None


def _run_git(cmd: list[str], cwd: Path | None = None) -> str:
    return subprocess.check_output(
        cmd,
        cwd=str(cwd) if cwd is not None else None,
        text=True,
        stderr=subprocess.DEVNULL,
    ).strip()


def read_git_info(cwd: Path | None = None) -> GitInfo:
    try:
        return GitInfo(
            available=True,
            repo_root=_run_git(["git", "rev-parse", "--show-toplevel"], cwd=cwd),
            commit=_run_git(["git", "rev-parse", "HEAD"], cwd=cwd),
            branch=_run_git(["git", "branch", "--show-current"], cwd=cwd),
            dirty=bool(_run_git(["git", "status", "--porcelain"], cwd=cwd)),
        )
    except Exception:
        return GitInfo(available=False)
