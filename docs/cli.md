# CLI

The entrypoint is `rau` with command set:

- rau check
- rau pack
- rau upload
- rau records

## New-machine setup (no sudo)

1. Clone and install the project:
   ```bash
   git clone <your-repo-url>
   cd research-artifact-uploader
   python -m venv .venv
   source .venv/bin/activate
   python -m pip install -e .
   ```

2. Install and configure `ossutil` (same as usual; if no sudo, install it under `~/.local/bin` and keep it on `PATH`).

3. Configure and validate:
   ```bash
   which ossutil
   ossutil --version
   ossutil config
   ossutil ls oss://luyukuan-research -e oss-accelerate.aliyuncs.com --region cn-shanghai
   ```

4. Run checks and upload:
   ```bash
   rau check --manifest examples/artifacts.yaml
   rau upload --manifest examples/artifacts.yaml --no-upload
   rau upload --manifest examples/artifacts.yaml
   ```

If list (`ossutil ls`) is denied by policy but upload is allowed, continue; `rau check` will treat list failure as a warning.
