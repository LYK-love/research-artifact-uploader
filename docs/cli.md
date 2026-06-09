# CLI

The entrypoint is `rau` with command set:

- rau check
- rau pack
- rau upload
- rau records

## New-machine setup

```bash
git clone <your-repo-url>
cd research-artifact-uploader
cargo build --release
cargo install --path . --root ~/.local
# binary -> ~/.local/bin/rau
cargo install --path .
```

## Install ossutil

- Alibaba Cloud OSSutil documentation: [https://www.alibabacloud.com/help/en/oss/developer-reference/ossutil](https://www.alibabacloud.com/help/en/oss/developer-reference/ossutil)
- Release assets: [https://github.com/aliyun/ossutil/releases](https://github.com/aliyun/ossutil/releases)

```bash
mkdir -p ~/.local/bin
cd /tmp
# download the right archive first
# unzip -o ossutil-<version>-<platform>.zip -d /tmp/ossutil_pkg
for f in /tmp/ossutil_pkg/ossutil*; do
  if [ -x "$f" ]; then
    install -m 0755 "$f" ~/.local/bin/ossutil
    break
  fi
done
export PATH="$HOME/.local/bin:$PATH"
ossutil config
```

If you prefer prebuilt artifacts, download from GitHub Releases and add to PATH.
No training command is needed for upload if the artifact files already exist.

## Direct upload workflow (no training command)

You can upload existing files directly:

```bash
cp examples/artifacts.yaml your_artifacts.yaml
# edit run.name, project, and artifact paths
rau check --manifest your_artifacts.yaml
rau upload --manifest your_artifacts.yaml
```

For a dry verification:

```bash
rau upload --manifest your_artifacts.yaml --no-upload
rau upload --manifest your_artifacts.yaml --dry-run
```

## Usage examples

```bash
rau check --manifest examples/artifacts.yaml
rau pack --manifest examples/artifacts.yaml
rau upload --manifest examples/artifacts.yaml --no-upload
rau upload --manifest examples/artifacts.yaml
rau records --jsonl docs/upload_records.jsonl --last 10
```
