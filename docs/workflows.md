# Workflows

Suggested local check and run sequence:

```bash
python -m pip install -e .
rau check --manifest examples/artifacts.yaml
rau pack --manifest examples/artifacts.yaml
rau upload --manifest examples/artifacts.yaml --no-upload
rau records --jsonl docs/upload_records.jsonl --last 5
```
