# CLI

Entrypoint:

```bash
rau
```

## Commands

```bash
rau check --manifest <file>
rau pack --manifest <file>
rau upload --manifest <file> [--dry-run|--no-upload|--no-record|--presign|--allow-outside-project]
rau records [--jsonl <file>] [--markdown <file>] [--last <N>]
```

### `check`

Validates manifest and local availability only. No side effects.

```bash
rau check --manifest examples/artifacts.yaml
```

### `pack`

Collects artifacts and writes:

- `.tar.gz`
- `.meta.json`
- `.sha256`

```bash
rau pack --manifest examples/artifacts.yaml
```

### `upload`

Full flow: collect -> pack -> metadata -> upload -> records.

```bash
rau upload --manifest examples/artifacts.yaml --no-upload
rau upload --manifest examples/artifacts.yaml
rau upload --manifest examples/artifacts.yaml --presign
```

### `records`

Print recent local records.

```bash
rau records --jsonl docs/upload_records.jsonl --last 10
rau records --markdown docs/upload_records.md --last 5
```
