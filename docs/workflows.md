# Workflows

Suggested local workflow:

```bash
cargo build --release
rau check --manifest examples/artifacts.yaml
rau pack --manifest examples/artifacts.yaml
rau upload --manifest examples/artifacts.yaml --no-upload
rau records --jsonl docs/upload_records.jsonl --last 5
rau upload --manifest examples/artifacts.yaml
```

When `--no-upload` is used, local metadata and archive are still generated for dry verification.
