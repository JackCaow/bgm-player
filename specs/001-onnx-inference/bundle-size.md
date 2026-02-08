# Bundle Size Comparison: ONNX vs PyTorch

**Feature**: 001-onnx-inference
**Date**: 2026-02-05

## Summary

Replacing PyTorch-based inference with ONNX Runtime significantly reduces the application bundle size.

## Size Comparison

| Component | PyTorch Bundle | ONNX Bundle | Savings |
|-----------|---------------|-------------|---------|
| ML Runtime | ~244 MB (PyTorch + deps) | ~35 MB (ONNX Runtime) | ~209 MB |
| Model File | ~83 MB (checkpoint) | ~83 MB (htdemucs_core.onnx) | 0 MB |
| Python Runtime | ~50 MB (bundled Python) | 0 MB | ~50 MB |
| **Total ML Components** | **~377 MB** | **~118 MB** | **~259 MB (69%)** |

## Notes

- ONNX Runtime is dynamically loaded, reducing initial binary size
- FP16 model variant available (~83 MB → ~83 MB, same size but faster inference)
- INT8 quantized model available (~51 MB) with minor quality tradeoff
- Python binary kept as fallback option (can be removed for pure ONNX builds)

## Verification

To verify bundle sizes after building:

```bash
# Build the app
npm run tauri:build

# Check bundle size (macOS)
du -sh src-tauri/target/release/bundle/macos/BGM\ Extractor.app

# Check individual components
ls -lh src-tauri/target/release/bundle/macos/BGM\ Extractor.app/Contents/Resources/
```

## Recommendations

1. **For minimum size**: Use ONNX mode exclusively, remove Python binary from `externalBin`
2. **For maximum compatibility**: Keep both modes, let users choose
3. **For best quality/size ratio**: Use FP16 ONNX model (same quality, faster inference)
