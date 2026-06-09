# research-artifact-uploader

> You can use AI to translate or explain this document and the rest of the project's documentation in your preferred language.
>
> 你可以使用 AI 将本文档和本项目的其他文档翻译成你偏好的语言，或为你解读其中的内容。
>
Therefore, repository usually does not provide translated parallel documents.

## What this project is

This project is a conservative uploader for research artifacts. It collects configured files, validates required inputs, packages them into tar.gz archives, and uploads the archive with checksums and metadata.

## How this project works

The tool reads a YAML manifest, checks artifact definitions, prepares an archive, computes SHA256, generates metadata, uploads using ossutil, and appends local upload records.

## What is not covered

The tool does not run training commands and does not contain any training harness.

## Installation

```bash
git clone <your-repo-url>
cd research-artifact-uploader
python -m venv .venv
source .venv/bin/activate
python -m pip install -e .
```

The project is intended to run as an unprivileged user. If you do not have `sudo`:
1. `ossutil` 可用标准方式安装；若无 `sudo`，请将二进制放到 `~/.local/bin` 并确保该目录在 `PATH`，其余步骤一致。

2. Verify `ossutil` is on `PATH`:
   ```bash
   which ossutil
   ossutil --version
   ```

3. Configure `ossutil` interactively once:
   ```bash
   ossutil config
   ```
   Fill in endpoint as `oss-accelerate.aliyuncs.com` and region as `cn-shanghai`.  
   Do not share AccessKey or SecretKey in shell history or logs.

4. Validate the OSS connection:
   ```bash
   ossutil ls oss://luyukuan-research -e oss-accelerate.aliyuncs.com --region cn-shanghai
   ```
   If list is not allowed by policy, the tool will continue with upload in `check`.

If you have shared Python environments, skip `python -m venv` and run against your existing environment.

## ossutil dependency

`ossutil` must already be installed and configured on the machine. This project invokes `ossutil cp` and does not manage secrets.

## Default OSS configuration

- bucket: luyukuan-research
- region: cn-shanghai
- endpoint: oss-accelerate.aliyuncs.com

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
  bucket: luyukuan-research
  region: cn-shanghai
  endpoint: oss-accelerate.aliyuncs.com
  remote_dir: artifacts/demo/demo_run

records:
  jsonl: docs/upload_records.jsonl
  markdown: docs/upload_records.md
```

## CLI usage

### check

```bash
rau check --manifest examples/artifacts.yaml
```

### pack

```bash
rau pack --manifest examples/artifacts.yaml
```

### upload

```bash
rau upload --manifest examples/artifacts.yaml --no-upload
rau upload --manifest examples/artifacts.yaml
```

### records

```bash
rau records --jsonl docs/upload_records.jsonl --last 10
```

## Security requirements

- Do not write AccessKey or secret key into manifest, metadata, records, logs, or docs.
- Use the project directory default restriction; pass --allow-outside-project only when needed.
- Prefer RAM users with least privileges.

## Troubleshooting

- region must be set in sign version 4
  - Ensure region is configured in the manifest as cn-shanghai.
- AccessDenied
  - Verify RAM permission for destination OSS path.
- The bucket you access does not belong to you
  - Confirm bucket owner and account context.
- ossutil not found
  - Install ossutil and ensure it is in PATH.

This project was written collaboratively by humans and AI.
