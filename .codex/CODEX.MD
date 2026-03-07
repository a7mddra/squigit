# Codex Workspace Notes

This repository keeps Codex automation assets under `.codex/` (not under `ui/`).

## Layout

- `.codex/tools/`: repo-level utility scripts
- `.codex/replacements/`: reusable replacement maps
- `.codex/reports/`: generated analysis outputs and patch previews

## Color Tokenizer

- Script: `.codex/tools/color_tokenizer.py`
- Source scan scope: `ui/src/**/*.module.css` and `ui/src/**/*.tsx`
- Existing token reuse source: `ui/src/styles/variables.css`

### Commands

```bash
python3 .codex/tools/color_tokenizer.py --report
python3 .codex/tools/color_tokenizer.py --test
python3 .codex/tools/color_tokenizer.py --apply
python3 .codex/tools/color_tokenizer.py --write
```

### Outputs

- `.codex/reports/color-map.json`
- `.codex/reports/color-report.md`
- `.codex/reports/color-tokenization.patch`
- On `--test`: preview files only
  - `ui/src/styles/variables.generated.css`
- On `--apply` / `--write`: updates source files in-place and appends new `--c-raw-xxx`
  tokens to both `:root` and `body.light-mode/html.light` in `ui/src/styles/variables.css`.

## Replacing Tool

- Script: `.codex/tools/replacing.py`
- Purpose: fast multi-file replacement with looped rules (VS Code replace style in CLI)

### Commands

```bash
# Dry-run with a replacement map
python3 .codex/tools/replacing.py \
  --map .codex/replacements/token-rename-v1.json \
  --include "ui/src/**/*.css" \
  --include "ui/src/**/*.ts" \
  --include "ui/src/**/*.tsx" \
  --exclude "**/*.generated.css" \
  --identifier-boundary

# Apply same replacement set
python3 .codex/tools/replacing.py \
  --map .codex/replacements/token-rename-v1.json \
  --include "ui/src/**/*.css" \
  --include "ui/src/**/*.ts" \
  --include "ui/src/**/*.tsx" \
  --exclude "**/*.generated.css" \
  --identifier-boundary \
  --apply
```
