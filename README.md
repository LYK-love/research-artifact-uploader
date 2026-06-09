# research-artifact-uploader

> You can use AI to translate or explain this document and the rest of the project's documentation in your preferred language.
>
> 你可以使用 AI 将本文档和本项目的其他文档翻译成你偏好的语言，或为你解读其中的内容。
> 因此仓库通常不再提供其他语言版本的平行文档。

## What this project is

`rau` is a conservative CLI that uploads research outputs described in a manifest to Alibaba Cloud OSS.

It validates artifact paths, builds a timestamped `tar.gz`, computes `sha256`, writes metadata, uploads via `ossutil cp`, and appends local upload records.

It does not run or manage training. Existing files can be uploaded directly.

```text
manifest.yml
   |
   v
+----------+
|  check  |
+----------+
   |
   v
+-------------+
|  collect    |
+-------------+
   |
   v
+----------------+
| pack + hash    |
+----------------+
   |
   v
+----------------------+
| upload metadata/uri  |
+----------------------+
   |
   v
+---------------------+
| local JSONL/Markdown |
+---------------------+
```

## Terminology

- **Manifest**: YAML file with run name, artifacts, archive options, and OSS destination.
- **Artifact**: A named `file`, `directory`, or `glob` entry to include.
- **run_id**: `<run_name>_<YYYYMMDD_HHMMSS>` for each upload/pack invocation.

## Prerequisites

- `ossutil` installed and configured.
- Rust toolchain (`cargo`, `rustc`).
- `git` installed for git-info optional metadata; not required.

No `sudo` is required if installed under `$HOME`.

## Install dependencies (required order)

### 1) Install `ossutil`

Official docs:
- https://www.alibabacloud.com/help/en/oss/developer-reference/ossutil
- https://github.com/aliyun/ossutil/releases

```bash
mkdir -p ~/.local/bin
# use the correct binary for your platform from official release
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
# verify
~/.local/bin/rau --help
```

## Default OSS settings

- bucket: `luyukuan-research`
- region: `cn-shanghai`
- endpoint: `oss-accelerate.aliyuncs.com`

## Install and use

```bash
rau check --manifest examples/artifacts.yaml
rau pack --manifest examples/artifacts.yaml
rau upload --manifest examples/artifacts.yaml --no-upload
rau upload --manifest examples/artifacts.yaml
rau records --jsonl docs/upload_records.jsonl --last 5
rau records --markdown docs/upload_records.md --last 5
```

### Key flags

- `--manifest <file>`: manifest path (required for `check/pack/upload`).
- `--dry-run`: show planned actions; no files written.
- `--no-upload`: pack + metadata only.
- `--no-record`: skip local logs.
- `--allow-outside-project`: allow paths outside current project directory.

### Manifest example

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
  bucket: luyukuan-research
  region: cn-shanghai
  endpoint: oss-accelerate.aliyuncs.com
  remote_dir: artifacts/demo/demo_run

records:
  jsonl: docs/upload_records.jsonl
  markdown: docs/upload_records.md
```

Defaults are used when `archive`, `oss`, or `records` sections are partially omitted.

## Non-training upload flow (direct file upload)

```bash
cp examples/artifacts.yaml your_artifacts.yaml
# edit only run.name / project / artifact paths
rau check --manifest your_artifacts.yaml
rau upload --manifest your_artifacts.yaml --no-upload
rau upload --manifest your_artifacts.yaml
```

## Security notes

- Do not store AccessKey/Secret in code, manifest, metadata, logs, or docs.
- Project-root paths outside current directory are blocked by default.
- Required artifacts missing cause failure.
- Required glob that matches nothing causes failure.
- `.git/`, `.rau/`, and `__pycache__/` are excluded unless explicitly declared.

## Troubleshooting

- `region must be set in sign version 4`
  - Ensure manifest has `region: cn-shanghai` (default is set).
- `AccessDenied`
  - Check RAM policy for `PutObject` / related path prefix.
- `The bucket you access does not belong to you`
  - Confirm the configured account owns the target bucket.
- `ossutil not found`
  - Confirm binary installed and in `PATH`.

This project was written collaboratively by humans and AI.
