# Implementation Plan: ONNX Model Inference

**Branch**: `001-onnx-inference` | **Date**: 2026-02-05 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-onnx-inference/spec.md`

## Summary

Replace PyTorch-based Demucs model inference with ONNX Runtime inference in Rust. The core ONNX inference engine (`onnx_demucs.rs`) is already implemented and tested. Remaining work focuses on frontend integration, progress reporting enhancement, and production bundling.

## Technical Context

**Language/Version**: Rust 1.75+ (backend), TypeScript/Vue 3 (frontend)
**Primary Dependencies**: ort 2.0.0-rc.11 (ONNX Runtime), Tauri 2.x, symphonia, rustfft, ndarray
**Storage**: File-based (audio files, ONNX models)
**Testing**: cargo test (Rust), example binaries for integration testing
**Target Platform**: macOS (primary), Windows, Linux (desktop)
**Project Type**: Desktop application (Tauri: Rust backend + Vue frontend)
**Performance Goals**: Processing time within 20% of PyTorch implementation, <5s startup
**Constraints**: Bundle size <50% of current, offline-capable, no Python runtime required
**Scale/Scope**: Single-user desktop app, audio files from seconds to hours

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Constitution is not yet configured for this project (template placeholders present). Proceeding without gate enforcement. Recommend running `/speckit.constitution` to establish project principles.

## Project Structure

### Documentation (this feature)

```text
specs/001-onnx-inference/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (Tauri command interfaces)
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
src-tauri/                    # Rust backend (Tauri)
├── src/
│   ├── lib.rs               # Tauri commands (extract_bgm, extract_bgm_onnx)
│   └── onnx_demucs.rs       # ONNX inference engine (STFT, model, audio I/O)
├── binaries/                # Platform-specific ONNX Runtime libraries
├── examples/
│   └── test_onnx.rs         # Integration test for ONNX inference
└── Cargo.toml               # Rust dependencies

src/                          # Vue 3 frontend
├── components/              # UI components (*View.vue pattern)
├── composables/             # Reactive logic (useFiles.ts, useSettings.ts)
├── utils/                   # Helper functions
└── i18n/                    # Internationalization

onnx_models/                  # ONNX model files
├── htdemucs_core.onnx       # Main model (166MB)
├── htdemucs_core_fp16.onnx  # Half precision (83MB)
└── htdemucs_config.json     # Model configuration

script/                       # Python utilities
├── export_onnx.py           # Model export script
└── onnx_inference.py        # Reference implementation
```

**Structure Decision**: Existing Tauri desktop app structure. Backend changes in `src-tauri/src/`, frontend changes in `src/composables/` and `src/components/`.

## Complexity Tracking

No constitution violations to justify - project follows existing patterns.
