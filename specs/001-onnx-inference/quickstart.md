# Quickstart: ONNX Model Inference

**Feature**: 001-onnx-inference
**Date**: 2026-02-05

## Prerequisites

- Rust 1.75+ with cargo
- Node.js 18+ with npm
- ONNX Runtime library (platform-specific)
- ONNX model file (htdemucs_core.onnx)

## Setup

### 1. Clone and Install Dependencies

```bash
# Install frontend dependencies
npm install

# Rust dependencies are handled by cargo
```

### 2. Download ONNX Runtime

The ONNX Runtime library must be placed in `src-tauri/binaries/`:

```bash
# macOS ARM64
python script/download_onnxruntime.py

# Or manually download from:
# https://github.com/microsoft/onnxruntime/releases
```

Expected file: `src-tauri/binaries/libonnxruntime-aarch64-apple-darwin.dylib`

### 3. Obtain ONNX Model

The htdemucs model should be in `onnx_models/`:

```bash
# Export from PyTorch (requires Python + Demucs)
python script/export_onnx.py

# Or download pre-exported model
```

Expected file: `onnx_models/htdemucs_core.onnx`

## Development

### Run the App

```bash
# Full desktop app with hot reload
npm run tauri:dev
```

### Test ONNX Inference Directly

```bash
# Run the Rust example
cd src-tauri
cargo run --example test_onnx -- /path/to/audio.mp3
```

### Build for Production

```bash
npm run tauri:build
```

## Key Files

| File | Purpose |
|------|---------|
| `src-tauri/src/onnx_demucs.rs` | ONNX inference engine |
| `src-tauri/src/lib.rs` | Tauri commands |
| `src/composables/useFiles.ts` | Frontend extraction logic |
| `onnx_models/htdemucs_core.onnx` | ONNX model |
| `script/export_onnx.py` | Model export script |

## Testing ONNX Mode

1. Start the app: `npm run tauri:dev`
2. Select an audio file (MP3, WAV, FLAC)
3. Enable ONNX mode in settings (when implemented)
4. Click extract and observe progress
5. Check output directory for separated stems

## Troubleshooting

### "ONNX Runtime not found"
- Verify `libonnxruntime*.dylib` exists in `src-tauri/binaries/`
- Check file permissions
- Ensure correct platform variant

### "Model file not found"
- Verify `htdemucs_core.onnx` exists in `onnx_models/`
- Run `python script/export_onnx.py` to generate

### "Inference failed"
- Check model compatibility with ONNX Runtime version
- Verify input audio is valid and decodable
- Check console for detailed error messages

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                    Vue 3 Frontend                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │ QueueView   │  │ ResultView  │  │ SettingsView│     │
│  └──────┬──────┘  └─────────────┘  └──────┬──────┘     │
│         │                                  │            │
│         ▼                                  ▼            │
│  ┌─────────────────────────────────────────────────┐   │
│  │              useFiles.ts composable              │   │
│  │         invoke("extract_bgm_onnx", ...)         │   │
│  └──────────────────────┬──────────────────────────┘   │
└─────────────────────────┼──────────────────────────────┘
                          │ Tauri IPC
┌─────────────────────────┼──────────────────────────────┐
│                    Rust Backend                        │
│  ┌──────────────────────▼──────────────────────────┐   │
│  │              lib.rs (Tauri commands)            │   │
│  │           extract_bgm_onnx() command            │   │
│  └──────────────────────┬──────────────────────────┘   │
│                         │                              │
│  ┌──────────────────────▼──────────────────────────┐   │
│  │           onnx_demucs.rs (inference)            │   │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────────────┐  │   │
│  │  │ STFT    │→ │ ONNX    │→ │ iSTFT + Fusion  │  │   │
│  │  │ (rustfft)│  │ Runtime │  │ (overlap-add)   │  │   │
│  │  └─────────┘  └─────────┘  └─────────────────┘  │   │
│  └─────────────────────────────────────────────────┘   │
└────────────────────────────────────────────────────────┘
```
