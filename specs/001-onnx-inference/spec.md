# Feature Specification: ONNX Model Inference

**Feature Branch**: `001-onnx-inference`
**Created**: 2026-02-05
**Status**: Draft
**Input**: User description: "目前模型转成 ONNX 替换掉 PyTorch"

## Overview

Replace the current PyTorch-based Demucs model inference with ONNX Runtime inference. This change aims to reduce application size, eliminate Python dependencies, and enable native Rust-based audio separation without requiring a bundled Python environment.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Extract BGM Using Native Inference (Priority: P1)

As a user, I want to extract background music from audio files using the application without needing Python or PyTorch installed on my system, so that the application is self-contained and easier to use.

**Why this priority**: This is the core functionality that enables the entire feature. Without native inference working, no other improvements matter.

**Independent Test**: Can be fully tested by selecting an audio file and running extraction - the output should contain separated audio tracks (vocals, drums, bass, other) with quality comparable to the original PyTorch implementation.

**Acceptance Scenarios**:

1. **Given** a user has an audio file (MP3, WAV, FLAC), **When** they initiate BGM extraction, **Then** the system processes the file using native inference and produces separated audio tracks
2. **Given** the extraction is in progress, **When** the user views the interface, **Then** they see accurate progress updates during processing
3. **Given** extraction completes, **When** the user checks the output, **Then** all expected audio stems (vocals, drums, bass, other) are available

---

### User Story 2 - Faster Application Startup (Priority: P2)

As a user, I want the application to start quickly without loading heavy machine learning frameworks, so that I can begin working immediately.

**Why this priority**: Improves user experience but is secondary to core functionality working correctly.

**Independent Test**: Can be tested by measuring application startup time - should launch and be ready for use within a reasonable timeframe.

**Acceptance Scenarios**:

1. **Given** the application is not running, **When** the user launches it, **Then** the main interface appears and is interactive within 5 seconds
2. **Given** the application has started, **When** the user selects a file for processing, **Then** inference can begin without additional loading delays

---

### User Story 3 - Smaller Application Bundle (Priority: P3)

As a user, I want a smaller application download and installation size, so that the application takes less disk space and downloads faster.

**Why this priority**: Nice-to-have improvement that follows naturally from removing Python/PyTorch dependencies.

**Independent Test**: Can be tested by comparing the final application bundle size before and after the change.

**Acceptance Scenarios**:

1. **Given** the application is built for distribution, **When** comparing to the previous version, **Then** the bundle size is significantly reduced (target: at least 50% smaller)

---

### Edge Cases

- What happens when the ONNX model file is missing or corrupted?
  - System displays a clear error message and prevents extraction from starting
- How does the system handle audio files that are too long for available memory?
  - System processes audio in chunks to manage memory usage
- What happens when the user cancels extraction mid-process?
  - System cleanly stops processing and releases resources
- How does the system handle unsupported audio formats?
  - System displays an error message listing supported formats

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST perform audio source separation using native inference without requiring Python runtime
- **FR-002**: System MUST support the same audio input formats as the current implementation (MP3, WAV, FLAC, M4A)
- **FR-003**: System MUST produce the same output stems as the current implementation (vocals, drums, bass, other)
- **FR-004**: System MUST display progress updates during extraction processing
- **FR-005**: System MUST allow users to cancel extraction at any point during processing
- **FR-006**: System MUST handle audio files of varying lengths (from seconds to hours)
- **FR-007**: System MUST report clear error messages when processing fails
- **FR-008**: System MUST support the htdemucs model (default model for best quality)

### Key Entities

- **ONNX Model**: Pre-trained Demucs model exported to ONNX format, used for audio source separation inference
- **Audio Stems**: The separated audio outputs (vocals, drums, bass, other) produced by the model
- **Processing Session**: A single extraction job with input file, progress state, and output stems

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Audio separation quality is comparable to the original implementation (output stems are audibly equivalent)
- **SC-002**: Application startup time is under 5 seconds on standard hardware
- **SC-003**: Application bundle size is reduced by at least 50% compared to the PyTorch-bundled version
- **SC-004**: Processing time for a 3-minute audio file is within 20% of the original implementation
- **SC-005**: 100% of existing audio format support is maintained (no regression in supported formats)

## Assumptions

- The htdemucs model can be successfully exported to ONNX format with acceptable quality
- ONNX Runtime provides sufficient performance for real-time-ish audio processing
- The target platforms (macOS, Windows, Linux) are supported by ONNX Runtime
- Users have hardware capable of running inference (modern CPU, optionally GPU)

## Out of Scope

- Supporting additional Demucs model variants beyond htdemucs in this initial implementation
- GPU acceleration (can be added in future iterations)
- Real-time streaming audio separation
- Model fine-tuning or training capabilities
