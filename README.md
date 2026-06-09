# research-artifact-uploader

> You can use AI to translate or explain this document and the rest of the project's documentation in your preferred language.
>
> 你可以使用 AI 将本文档和本项目的其他文档翻译成你偏好的语言，或为你解读其中的内容。
> 因此仓库通常不再提供其他语言版本的平行文档。

## What this project is

`rau` uploads deep-learning experiment artifacts to Alibaba Cloud OSS.

Use it after training ends: point a manifest at existing outputs and run `rau upload`.

It reads the manifest, validates required/optional artifact rules, packages matched files into a timestamped archive, computes checksums, and uploads archive + metadata.

It is conservative by design: no implicit network writes, no credential handling, and no training orchestration.

## Minimal flow

```text
experiment outputs
    |
    v
manifest.yml
    |
    v
+---------+
| validate |
+---------+
    |
    v
+---------+
| collect  |
+---------+
    |
    v
+------------+
| pack+meta  |
+------------+
    |
    v
+------------+
| upload 3x  |
+------------+
    |
    v
+----------------+
| local records   |
+----------------+
```

## Why for deep-learning workflows

`rau` is useful both for direct file upload and for post-training automation.

- In a normal research loop, define `examples/artifacts.yaml` (or a copy) pointing to your output paths.
- After training ends, call `rau upload` with that manifest.

```bash
# after long-running training
python train.py --config cfg.yaml
rau upload --manifest my_run_artifacts.yaml
```

If no training command exists, existing finished outputs can also be uploaded directly.

## Terminology

- **Manifest**: YAML description of the run, artifact list, archive strategy, and OSS destination.
- **Artifact**: One `file`, `directory`, or `glob` entry to include.
- **run_id**: `<run_name>_<YYYYMMDD_HHMMSS>` generated per run.

## Prerequisites

- `ossutil` binary installed and configured.
- Rust toolchain (`cargo`, `rustc`).
- `git` (optional; metadata will mark git info unavailable when absent).

No `sudo` is required if installing under `$HOME`.

## Install dependencies (required order)

### 1) Install `ossutil` first

Official docs:
- https://www.alibabacloud.com/help/en/oss/developer-reference/ossutil
- https://github.com/aliyun/ossutil/releases

```bash
mkdir -p ~/.local/bin
curl -L -o ~/.local/bin/ossutil <ossutil-download-url>
chmod +x ~/.local/bin/ossutil
export PATH="$HOME/.local/bin:$PATH"
ossutil config
```

### 2) Install `rau`

```bash
git clone <your-repo-url>
cd research-artifact-uploader

cargo build --release
# optional: install to PATH
cargo install --path . --root ~/.local
~/.local/bin/rau --help
```

## OSS fields in manifest

Use placeholders in your own docs/manifests to avoid hardcoding shared deployment details:

- `bucket: <your_bucket>`
- `region: <your_region>`
- `endpoint: <your_endpoint>`
- `remote_dir: <your_remote_path>`

## Basic usage

```bash
rau check --manifest examples/artifacts.yaml
rau pack --manifest examples/artifacts.yaml
rau upload --manifest examples/artifacts.yaml --no-upload
rau upload --manifest examples/artifacts.yaml
rau records --jsonl docs/upload_records.jsonl --last 5
rau records --markdown docs/upload_records.md --last 5
```

## Key flags

- `--manifest <file>`: manifest file path (required for `check`, `pack`, `upload`).
- `--dry-run`: plan only, no files created/uploaded/recorded.
- `--no-upload`: create archive + metadata only.
- `--no-record`: skip local JSONL/Markdown append.
- `--allow-outside-project`: allow artifact paths outside current working directory.

## Manifest example

```yaml
run:
  name: demo_run
  project: research-artifact-uploader-demo
  tags:
    - demo

artifacts:
  - name: ckpt_latest
    path: examples/fake_run/ckpt/latest
    type: directory
    required: true

  - name: metrics
    path: examples/fake_run/metrics.jsonl
    type: file
    required: true

  - name: videos
    path: examples/fake_run/videos/*.mp4
    type: glob
    required: false

archive:
  output_dir: .rau/archives
  format: tar.gz
  compression_level: 3
  include_manifest: true
  include_git_info: true
  include_metadata: true

oss:
  bucket: <your_bucket>
  region: <your_region>
  endpoint: <your_endpoint>
  remote_dir: artifacts/demo/demo_run

records:
  jsonl: docs/upload_records.jsonl
  markdown: docs/upload_records.md
```

Defaults apply if `archive`, `oss`, or `records` sections are omitted.

## No-training direct upload flow

```bash
cp examples/artifacts.yaml my_artifacts.yaml
# adjust run.name / project / artifact paths
rau check --manifest my_artifacts.yaml
rau upload --manifest my_artifacts.yaml --no-upload
rau upload --manifest my_artifacts.yaml
```

## Security notes

- Never place credentials in code, manifests, metadata, logs, or readme.
- Default path check blocks artifact locations outside the current project.
- Missing required artifacts fail the run.
- Required `glob` with zero matches fails the run.
- `.git/`, `.rau/`, and `__pycache__/` are excluded unless explicitly included.

## Troubleshooting

- `region must be set in sign version 4`
  - Ensure manifest includes `region` and `ossutil` is using the correct profile.
- `AccessDenied`
  - Check OSS policy for destination prefix and `PutObject` permission.
- `The bucket you access does not belong to you`
  - Confirm you are using the owning account/project credentials.
- `ossutil not found`
  - Ensure `ossutil` is on `PATH`.

This project was written collaboratively by humans and AI.
