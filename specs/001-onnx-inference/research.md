# Research: ONNX Model Inference

**Feature**: 001-onnx-inference
**Date**: 2026-02-05

## Executive Summary

Research confirms that ONNX-based inference is viable and largely implemented. The core inference engine exists in `src-tauri/src/onnx_demucs.rs` with working STFT/iSTFT, model loading, and audio I/O. Remaining work is integration, not research.

---

## Research Topics

### 1. ONNX Runtime Integration in Rust

**Decision**: Use `ort` crate v2.0.0-rc.11 with dynamic loading

**Rationale**:
- `ort` is the official Rust bindings for ONNX Runtime
- Dynamic loading (`load-dynamic` feature) allows bundling the library separately
- CoreML support available via feature flag for macOS GPU acceleration
- Already integrated and tested in the codebase

**Alternatives Considered**:
- `tract` (pure Rust): Rejected - incomplete ONNX operator support for Demucs
- Static linking: Rejected - increases binary size significantly
- Python subprocess: Rejected - defeats the purpose of removing Python dependency

---

### 2. Demucs Model Export to ONNX

**Decision**: Export htdemucs model using PyTorch's torch.onnx.export

**Rationale**:
- htdemucs is the default/best quality model
- Export script already exists (`script/export_onnx.py`)
- Multiple model variants available (FP32, FP16, INT8)
- Model configuration captured in `htdemucs_config.json`

**Model Specifications**:
- Sample rate: 44100 Hz
- Channels: 2 (stereo)
- Sources: drums, bass, other, vocals
- NFFT: 4096, Hop length: 1024
- Frequency bins: 2049

**Alternatives Considered**:
- Export all Demucs variants: Deferred - htdemucs sufficient for initial release
- ONNX optimization passes: Available (htdemucs_core_optimized.onnx) but larger file

---

### 3. Audio Processing Pipeline in Rust

**Decision**: Custom STFT/iSTFT implementation matching PyTorch behavior

**Rationale**:
- PyTorch's STFT has specific behaviors (center padding, normalization)
- Existing implementation in `onnx_demucs.rs` verified against Python reference
- Uses `rustfft` for FFT operations, `apodize` for window functions
- Handles reflect padding, Hann window, overlap-add reconstruction

**Key Implementation Details**:
- Hann window with periodic mode
- Reflect padding for center=True
- Normalized FFT (1/sqrt(nfft))
- CAC mode: complex as channels (real/imag interleaved)

**Alternatives Considered**:
- Use existing audio DSP crate: Rejected - none match PyTorch's exact behavior
- Pre-compute STFT in Python: Rejected - adds complexity, defeats purpose

---

### 4. Long Audio Handling

**Decision**: Segment-based processing with overlap-add

**Rationale**:
- Memory constraints prevent processing very long audio at once
- Implemented in `OnnxDemucs::separate_long()` method
- 25% overlap between segments with triangular window blending
- Configurable segment size (default: model's native segment length)

**Alternatives Considered**:
- Streaming processing: Rejected - Demucs architecture not suited for streaming
- Memory mapping: Rejected - doesn't solve GPU memory constraints

---

### 5. Platform-Specific Execution Providers

**Decision**: CPU default, CoreML on macOS, CUDA optional

**Rationale**:
- CPU works everywhere, acceptable performance for audio processing
- CoreML provides GPU acceleration on macOS without CUDA
- CUDA support available but requires user to have NVIDIA GPU + drivers

**Implementation**:
- Execution provider selection in `OnnxDemucs::new()`
- CoreML enabled via `ort` feature flag
- Graceful fallback if preferred provider unavailable

**Alternatives Considered**:
- DirectML for Windows: Deferred - CPU sufficient for initial release
- Metal directly: Rejected - CoreML provides better abstraction

---

### 6. Audio Format Support

**Decision**: Use Symphonia for decoding, Hound for WAV output

**Rationale**:
- Symphonia supports MP3, AAC, FLAC, OGG, WAV (matches current support)
- Pure Rust, no external dependencies
- Hound provides simple WAV writing (16-bit PCM)
- Linear resampling to 44.1kHz for model compatibility

**Alternatives Considered**:
- FFmpeg bindings: Rejected - adds external dependency
- rodio: Rejected - playback-focused, not suitable for processing

---

### 7. Progress Reporting

**Decision**: Segment-level progress events via Tauri event system

**Rationale**:
- Existing `extraction-progress` event structure compatible
- Progress calculated as: (completed_segments / total_segments) * 100
- Stage information: "loading", "processing", "saving"

**Current Gap**: Progress callbacks not yet wired into `OnnxDemucs::separate()` method

---

### 8. Bundle Size Analysis

**Decision**: Dynamic ONNX Runtime + compressed model achieves >50% reduction

**Current Bundle Estimate**:
- ONNX Runtime library: ~34MB (vs PyTorch ~244MB)
- ONNX model (FP16): ~83MB (vs PyTorch checkpoint ~83MB)
- Total reduction: ~200MB+ savings

**Alternatives Considered**:
- INT8 quantized model: Available (51MB) but quality tradeoff
- Model download on first run: Deferred - complicates offline use

---

## Unresolved Questions

None - all technical questions resolved through existing implementation and testing.

## References

- ONNX Runtime Rust bindings: https://github.com/pykeio/ort
- Demucs paper: https://arxiv.org/abs/2111.03600
- Symphonia audio library: https://github.com/pdeljanov/Symphonia
- Existing implementation: `src-tauri/src/onnx_demucs.rs`
