---
name: demucs-onnx-export
description: Export Demucs models to ONNX (including BagOfModels ensembles like htdemucs_ft) and integrate/bundle them in this get-bgm (Vue + Tauri) repo.
---

# Demucs → ONNX → get-bgm workflow

Use this skill when you need to:
- Export `demucs` pretrained models to ONNX (e.g. `htdemucs`, `htdemucs_6s`, `htdemucs_ft`).
- Sync exported artifacts into `src-tauri/onnx_models/` for Tauri bundling.
- Wire a new model option end-to-end (Rust backend routing + Vue settings + i18n).

## Preconditions

- Python env with `torch`, `demucs`, `onnx`, `onnxruntime` installed.
  - In this repo we typically use: `~/miniconda3/bin/python3`
- Repo layout:
  - Export outputs: `onnx_models/`
  - Bundled resources (copied by build script): `src-tauri/onnx_models/`

## Export ONNX

Run:
- `~/miniconda3/bin/python3 script/export_onnx.py -m htdemucs -o onnx_models --verify --test`
- `~/miniconda3/bin/python3 script/export_onnx.py -m htdemucs_6s -o onnx_models --verify --test`
- `~/miniconda3/bin/python3 script/export_onnx.py -m htdemucs_ft -o onnx_models --verify --test`

Notes:
- `htdemucs_ft` is a `BagOfModels` (ensemble). The exporter will create:
  - `onnx_models/htdemucs_ft_core_0.onnx` … `_3.onnx`
  - `onnx_models/htdemucs_ft_config.json` with `ensemble.models` + `ensemble.weights`
- For ensemble models, do not keep a stale `*_core.onnx` around (avoid bundling confusion).

## Sync artifacts into Tauri bundle directory

Copy the exported files into `src-tauri/onnx_models/` (Tauri bundles from inside `src-tauri/`):

- Single-model examples:
  - `cp -f onnx_models/htdemucs_core.onnx onnx_models/htdemucs_config.json src-tauri/onnx_models/`
  - `cp -f onnx_models/htdemucs_6s_core.onnx onnx_models/htdemucs_6s_config.json src-tauri/onnx_models/`
- Ensemble example:
  - `cp -f onnx_models/htdemucs_ft_core_0.onnx onnx_models/htdemucs_ft_core_1.onnx onnx_models/htdemucs_ft_core_2.onnx onnx_models/htdemucs_ft_core_3.onnx onnx_models/htdemucs_ft_config.json src-tauri/onnx_models/`

## Backend wiring (Rust / Tauri)

Checklist:
- Map model key → filename(s):
  - `src-tauri/src/lib.rs`:
    - `get_onnx_model_filename(model_key)`
    - `get_onnx_config_filename(model_key)`
- Route the UI model value to `extract_bgm_onnx(...)`:
  - `src-tauri/src/lib.rs` in `extract_bgm(...)`
- If the config has `ensemble`, the backend should:
  - Load all `ensemble.models` in the same directory as the base model file.
  - Apply per-stem weights (`ensemble.weights`) when combining outputs.
  - (Already implemented in this repo’s `OnnxDemucs`.)

Bundling:
- `src-tauri/build.rs` copies a fixed allowlist of model files from repo root `onnx_models/` → `src-tauri/onnx_models/`.
  - If you add a new model (or add ensemble cores), update this list.
- `src-tauri/tauri.conf.json` bundles `onnx_models/*.onnx` + `onnx_models/*.json`.

## Frontend wiring (Vue)

Checklist:
- Add model option in settings:
  - `src/components/SettingsView.vue` → `modelOptions`
  - Ensure track support mapping is correct (2/4/6 track).
- Normalize stored legacy values:
  - `src/composables/useModelSettings.ts` → `normalizeModel`
- Add i18n copy:
  - `src/i18n/index.ts` → `model.<your_key>.desc`

## Validate

- Typecheck + build UI: `npm run build`
- Rust check (offline): `cd src-tauri && cargo check --offline --release`
- Quick ONNX run (saves stems):
  - `cd src-tauri && cargo run --example test_onnx --offline --release -- --model htdemucs --input "/path/to/file.mp3" --seconds 15 --save`
  - `cd src-tauri && cargo run --example test_onnx --offline --release -- --model htdemucs_ft --input "/path/to/file.mp3" --seconds 15 --save`

## Package

- Build app + DMG: `npm run tauri:build`

## Troubleshooting

- UI progress “stuck at 5%”:
  - Usually means progress callbacks are not forwarded during pre-processing or ensemble runs.
  - Verify progress updates inside `src-tauri/src/onnx_demucs.rs` inference loop.
- `htdemucs_ft` sounds wrong vs PyTorch:
  - Ensure you exported all 4 sub-models and the config contains `ensemble`.
  - Ensure the app is bundling `htdemucs_ft_core_0..3.onnx` (not just one).
