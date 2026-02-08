# Data Model: ONNX Model Inference

**Feature**: 001-onnx-inference
**Date**: 2026-02-05

## Entities

### 1. OnnxDemucs

**Description**: Core inference engine that loads ONNX model and performs audio source separation.

**Fields**:
| Field | Type | Description |
|-------|------|-------------|
| session | ort::Session | ONNX Runtime session with loaded model |
| sample_rate | u32 | Audio sample rate (44100 Hz) |
| nfft | usize | FFT size (4096) |
| hop_length | usize | STFT hop length (1024) |
| segment | usize | Processing segment length in samples |
| sources | Vec<String> | Output source names ["drums", "bass", "other", "vocals"] |

**State Transitions**: None (stateless inference)

**Validation Rules**:
- Model file must exist and be valid ONNX format
- ONNX Runtime library must be loadable
- Input audio must be decodable to PCM

---

### 2. ProcessingSession

**Description**: Represents a single BGM extraction job with progress tracking.

**Fields**:
| Field | Type | Description |
|-------|------|-------------|
| input_path | String | Path to input audio file |
| output_dir | String | Directory for output stems |
| model | String | Model name (e.g., "htdemucs") |
| format | String | Output format ("wav", "mp3", "flac") |
| progress | u8 | Current progress percentage (0-100) |
| stage | String | Current stage ("loading", "processing", "saving") |
| status | String | Status message for UI display |
| cancelled | bool | Whether user requested cancellation |

**State Transitions**:
```
Pending → Loading → Processing → Saving → Completed
                ↓           ↓
              Failed      Cancelled
```

**Validation Rules**:
- input_path must exist and be readable
- output_dir must be writable
- format must be one of: wav, mp3, flac, m4a

---

### 3. AudioStem

**Description**: A separated audio source output from the model.

**Fields**:
| Field | Type | Description |
|-------|------|-------------|
| name | String | Source name (drums, bass, other, vocals) |
| path | String | Output file path |
| samples | Vec<f32> | Audio samples (stereo interleaved) |
| sample_rate | u32 | Sample rate (44100 Hz) |
| channels | u8 | Number of channels (2) |

**Validation Rules**:
- name must be one of the model's defined sources
- samples length must be divisible by channels

---

### 4. ExtractResult

**Description**: Result returned from extraction command to frontend.

**Fields**:
| Field | Type | Description |
|-------|------|-------------|
| success | bool | Whether extraction completed successfully |
| output_dir | String | Directory containing output files |
| stems | Vec<String> | List of generated stem file paths |
| error | Option<String> | Error message if failed |

---

## Relationships

```
ProcessingSession 1──────* AudioStem
       │
       │ uses
       ▼
   OnnxDemucs
       │
       │ produces
       ▼
  ExtractResult
```

## Data Flow

```
Input Audio File
       │
       ▼
┌─────────────────┐
│ Symphonia       │ Decode to PCM
│ Decoder         │ Resample to 44.1kHz
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ STFT            │ Time → Frequency domain
│ (Rust FFT)      │ Complex spectrogram
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ ONNX Model      │ Spectral + time masks
│ (htdemucs)      │ 4 source separation
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ iSTFT           │ Frequency → Time domain
│ + Fusion        │ Overlap-add reconstruction
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Hound           │ Write WAV files
│ Writer          │ 16-bit PCM output
└────────┬────────┘
         │
         ▼
   4 Audio Stems
   (drums, bass, other, vocals)
```
