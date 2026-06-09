# research-artifact-uploader

> You can use AI to translate or explain this document and the rest of the project's documentation in your preferred language.
>
> 你可以使用 AI 将本文档和本项目的其他文档翻译成你偏好的语言，或为你解读其中的内容。
>
> Therefore, repository usually does not provide translated parallel documents.

## What this project is

This project is a conservative uploader for research artifacts. It collects files described in a YAML manifest, validates required inputs, packages them into tar.gz archives, uploads them to Alibaba Cloud OSS with `ossutil`, and records metadata + checksums.

It does not run training commands. It can upload existing files directly as-is if you only need to package/record files from a finished experiment.

## How this project works

- `rau check`: validate manifest and upload target checks.
- `rau pack`: collect and create `.tar.gz` + `metadata` + `sha256` locally.
- `rau upload`: full flow (pack, upload 3 files, append records).
- `rau records`: print recent upload records from JSONL.

If you already have finished experiment outputs, you can upload them directly. No training command is required for this tool.

## Installation

### Install from source (recommended for Linux/macOS)

```bash
# clone repo
git clone <your-repo-url>
cd research-artifact-uploader

# build and install for current user
cargo build --release
cargo install --path . --root ~/.local
```

Binary path: `~/.local/bin/rau`

If your environment has no `sudo`, this still works because it installs to your home directory.

### Download prebuilt binary

If a release is published, download the matching archive from GitHub Releases and add it to PATH. The binary artifact is named by architecture and platform, for example:

`rau-<version>-x86_64-unknown-linux-gnu.tar.gz`

Example:

```bash
chmod +x rau-x86_64-unknown-linux-gnu
mkdir -p ~/.local/bin
mv rau-x86_64-unknown-linux-gnu ~/.local/bin/rau
export PATH="$HOME/.local/bin:$PATH"
```

## ossutil dependency

`ossutil` must be installed and configured on the host machine. This project only invokes `ossutil cp` with:

```bash
ossutil cp <local> <oss-uri> -e oss-accelerate.aliyuncs.com --region cn-shanghai
```

Install `ossutil` (official docs):

- Alibaba Cloud OSSutil documentation: [https://www.alibabacloud.com/help/en/oss/developer-reference/ossutil](https://www.alibabacloud.com/help/en/oss/developer-reference/ossutil)
- GitHub releases (pick correct package): [https://github.com/aliyun/ossutil/releases](https://github.com/aliyun/ossutil/releases)

Minimal install (Linux/macOS, no `sudo`), with downloaded package from the release page:

```bash
mkdir -p ~/.local/bin
cd /tmp

# example placeholder: ossutil-v1.7.20-linux-amd64.zip
# download the correct asset for your OS/arch first.
unzip -o ossutil-v1.7.20-linux-amd64.zip -d /tmp/ossutil_pkg
for f in /tmp/ossutil_pkg/ossutil*; do
  if [ -x "$f" ]; then
    install -m 0755 "$f" ~/.local/bin/ossutil
    break
  fi
done
export PATH="$HOME/.local/bin:$PATH"
rm -rf /tmp/ossutil_pkg

ossutil config
```

Default OSS configuration:

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

## Usage

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

## Direct upload in practice

If you do not want to run a new training job, you can still upload existing files:

1. Prepare a manifest whose `artifacts.path` points to existing files/directories.
2. Run `rau check --manifest <your_manifest>`.
3. Run `rau upload --manifest <your_manifest>`.

No training command is required in this flow.

## Security requirements

- Do not write AccessKey or secret keys into manifest, metadata, records, docs, or logs.
- Keep outputs outside of default project boundaries unless explicitly allowed.
- Use RAM users and least privilege policies.

## Troubleshooting

- region must be set in sign version 4
  - Ensure region is set as `cn-shanghai` in manifest.
- AccessDenied
  - Confirm IAM policy for destination path.
- The bucket you access does not belong to you
  - Confirm account and bucket ownership.
- ossutil not found
  - Ensure `ossutil` is installed and in PATH.

This project was written collaboratively by humans and AI.
