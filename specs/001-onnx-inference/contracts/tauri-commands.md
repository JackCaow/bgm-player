# Tauri Command Contracts: ONNX Model Inference

**Feature**: 001-onnx-inference
**Date**: 2026-02-05

## Commands

### extract_bgm_onnx

**Description**: Extract BGM from audio file using ONNX Runtime inference.

**Signature** (Rust):
```rust
#[tauri::command]
async fn extract_bgm_onnx(
    app: AppHandle,
    input_path: String,
    output_dir: String,
    model: String,
    format: String,
    separation_mode: String,
) -> Result<ExtractResult, String>
```

**Parameters**:
| Name | Type | Required | Description |
|------|------|----------|-------------|
| input_path | String | Yes | Absolute path to input audio file |
| output_dir | String | Yes | Directory for output stems |
| model | String | Yes | Model name ("htdemucs") |
| format | String | Yes | Output format ("wav", "mp3", "flac", "m4a") |
| separation_mode | String | Yes | "2track", "4track", or "6track" |

**Response**:
```typescript
interface ExtractResult {
  success: boolean;
  output_dir: string;
  stems: string[];      // Paths to generated files
  error?: string;       // Present if success=false
}
```

**Events Emitted**:
```typescript
// Event: "extraction-progress"
interface ProgressPayload {
  progress: number;     // 0-100
  status: string;       // Human-readable status
  stage: string;        // "loading" | "processing" | "saving"
}
```

**Error Conditions**:
| Error | Description |
|-------|-------------|
| "ONNX Runtime not found" | Library not bundled or loadable |
| "Model file not found" | ONNX model missing from expected path |
| "Unsupported audio format" | Input file format not supported |
| "Failed to decode audio" | Corrupted or invalid audio file |
| "Inference failed" | ONNX model execution error |
| "Failed to write output" | Cannot write to output directory |

---

### cancel_extraction (existing)

**Description**: Cancel an in-progress extraction.

**Signature** (Rust):
```rust
#[tauri::command]
fn cancel_extraction() -> Result<(), String>
```

**Behavior**: Sets cancellation flag checked during segment processing.

---

## Frontend Integration

### TypeScript Invocation

```typescript
// src/composables/useFiles.ts
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface ExtractResult {
  success: boolean;
  output_dir: string;
  stems: string[];
  error?: string;
}

interface ProgressPayload {
  progress: number;
  status: string;
  stage: string;
}

async function extractWithOnnx(
  inputPath: string,
  outputDir: string,
  model: string = "htdemucs",
  format: string = "wav",
  separationMode: string = "4track"
): Promise<ExtractResult> {
  // Listen for progress events
  const unlisten = await listen<ProgressPayload>(
    "extraction-progress",
    (event) => {
      console.log(`Progress: ${event.payload.progress}%`);
      console.log(`Stage: ${event.payload.stage}`);
      console.log(`Status: ${event.payload.status}`);
    }
  );

  try {
    const result = await invoke<ExtractResult>("extract_bgm_onnx", {
      inputPath,
      outputDir,
      model,
      format,
      separationMode,
    });
    return result;
  } finally {
    unlisten();
  }
}
```

---

## Settings Contract

### ONNX Mode Toggle

```typescript
// Settings storage key
const ONNX_MODE_KEY = "use_onnx_inference";

interface OnnxSettings {
  enabled: boolean;           // Use ONNX instead of Python
  modelVariant: string;       // "htdemucs" | "htdemucs_fp16"
  executionProvider: string;  // "cpu" | "coreml" | "cuda"
}
```

---

## Resource Paths

### Model Files (Production)
```
Contents/Resources/
├── onnx_models/
│   ├── htdemucs_core.onnx
│   └── htdemucs_config.json
└── lib/
    └── libonnxruntime.dylib
```

### Model Files (Development)
```
onnx_models/
├── htdemucs_core.onnx
└── htdemucs_config.json

src-tauri/binaries/
└── libonnxruntime-{target}.dylib
```
