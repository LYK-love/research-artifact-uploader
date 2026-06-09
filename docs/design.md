# Design

## Scope

`rau` is a conservative, auditable uploader with minimal side effects:

1. Validate manifest and artifact paths.
2. Collect artifacts (with required/optional semantics).
3. Pack deterministic tar.gz snapshot under `.rau/archives`.
4. Compute SHA256 and write metadata.
5. Upload through `ossutil cp` and record local JSONL/Markdown summaries.

## Core data flow

- `Manifest` -> collect -> build archive -> compute sha256 -> metadata -> upload -> local records.
- Every run has `run_name_YYYYMMDD_HHMMSS`.

## Default constraints

- Relative artifact paths are interpreted from current working directory.
- Project-root exclusion defaults for `.git`, `.rau`, `__pycache__`.
- Upload URI is `${remote_dir}/${timestamp}/`.

## Failure behavior

- Any failure before upload aborts and prints an actionable error.
- `--dry-run` and `--no-upload` avoid external side effects.
- Optional artifact misses only warn.
