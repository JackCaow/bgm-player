# Tasks: ONNX Model Inference

**Input**: Design documents from `/specs/001-onnx-inference/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Not explicitly requested - test tasks omitted.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Backend**: `src-tauri/src/` (Rust/Tauri)
- **Frontend**: `src/` (Vue 3/TypeScript)
- **Models**: `onnx_models/`
- **Scripts**: `script/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Verify existing ONNX infrastructure and prepare for integration

- [x] T001 Verify ONNX Runtime library exists in src-tauri/binaries/libonnxruntime-aarch64-apple-darwin.dylib
- [x] T002 Verify ONNX model exists in onnx_models/htdemucs_core.onnx
- [x] T003 [P] Verify htdemucs_config.json matches model parameters in onnx_models/htdemucs_config.json
- [x] T004 [P] Run existing test example to confirm ONNX inference works: cargo run --example test_onnx in src-tauri/

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core backend enhancements that MUST be complete before frontend integration

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T005 Add progress callback parameter to OnnxDemucs::separate() method in src-tauri/src/onnx_demucs.rs
- [x] T006 Add progress callback parameter to OnnxDemucs::separate_long() method in src-tauri/src/onnx_demucs.rs
- [x] T007 Implement segment-level progress calculation in separate_long() in src-tauri/src/onnx_demucs.rs
- [x] T008 Add cancellation check between segments in separate_long() in src-tauri/src/onnx_demucs.rs
- [x] T009 Wire progress events to Tauri event system in extract_bgm_onnx command in src-tauri/src/lib.rs

**Checkpoint**: Foundation ready - ONNX inference has progress reporting and cancellation support

---

## Phase 3: User Story 1 - Extract BGM Using Native Inference (Priority: P1) 🎯 MVP

**Goal**: Users can extract BGM from audio files using ONNX inference without Python

**Independent Test**: Select an audio file (MP3/WAV/FLAC), run extraction, verify 4 output stems (drums, bass, other, vocals) are produced with comparable quality to Python version

### Implementation for User Story 1

- [x] T010 [US1] Create useOnnxSettings.ts composable for ONNX mode state in src/composables/useOnnxSettings.ts
- [x] T011 [US1] Add ONNX mode toggle switch to SettingsView.vue in src/components/SettingsView.vue
- [x] T012 [US1] Add i18n strings for ONNX settings (Chinese and English) in src/i18n/
- [x] T013 [US1] Update useFiles.ts to conditionally call extract_bgm_onnx when ONNX mode enabled in src/composables/useFiles.ts
- [x] T014 [US1] Handle ONNX-specific error messages in extraction flow in src/composables/useFiles.ts
- [x] T015 [US1] Add model file validation on app startup in src-tauri/src/lib.rs
- [x] T016 [US1] Add ONNX Runtime library validation on app startup in src-tauri/src/lib.rs
- [x] T017 [US1] Display clear error if ONNX resources missing in src/composables/useFiles.ts

**Checkpoint**: User Story 1 complete - users can toggle ONNX mode and extract BGM natively

---

## Phase 4: User Story 2 - Faster Application Startup (Priority: P2)

**Goal**: Application starts within 5 seconds without loading heavy ML frameworks

**Independent Test**: Measure cold start time from app launch to interactive UI - should be under 5 seconds

### Implementation for User Story 2

- [x] T018 [US2] Implement lazy loading for ONNX model (load on first extraction, not startup) in src-tauri/src/onnx_demucs.rs
- [x] T019 [US2] Add model loading indicator to UI when first extraction starts in src/composables/useFiles.ts
- [x] T020 [US2] Cache loaded ONNX session for subsequent extractions in src-tauri/src/lib.rs
- [x] T021 [US2] Add "loading" stage to progress events during model initialization in src-tauri/src/lib.rs

**Checkpoint**: User Story 2 complete - app starts quickly, model loads on demand

---

## Phase 5: User Story 3 - Smaller Application Bundle (Priority: P3)

**Goal**: Application bundle size reduced by at least 50% compared to PyTorch version

**Independent Test**: Build production bundle, compare size to previous PyTorch-bundled version

### Implementation for User Story 3

- [x] T022 [US3] Configure tauri.conf.json to bundle ONNX Runtime library in src-tauri/tauri.conf.json
- [x] T023 [US3] Configure tauri.conf.json to bundle ONNX model files in src-tauri/tauri.conf.json
- [x] T024 [US3] Add resource path resolution for production builds in src-tauri/src/lib.rs
- [x] T025 [US3] Remove Python/PyInstaller binary bundling from tauri.conf.json in src-tauri/tauri.conf.json
- [x] T026 [US3] Update build scripts to exclude Python dependencies in package.json
- [x] T027 [US3] Document bundle size comparison in specs/001-onnx-inference/

**Checkpoint**: User Story 3 complete - production bundle is significantly smaller

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [x] T028 [P] Add fallback to Python extraction if ONNX fails in src-tauri/src/lib.rs
- [x] T029 [P] Add execution provider selection (CPU/CoreML) to settings in src/components/SettingsView.vue
- [x] T030 Verify all supported audio formats work with ONNX inference (MP3, WAV, FLAC, M4A)
- [x] T031 Compare output quality between ONNX and Python implementations
- [x] T032 [P] Update CLAUDE.md with ONNX-specific development notes
- [x] T033 Run quickstart.md validation to ensure setup instructions are accurate

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-5)**: All depend on Foundational phase completion
  - User stories can proceed in priority order (P1 → P2 → P3)
  - US2 and US3 can start in parallel after US1 if desired
- **Polish (Phase 6)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - Core functionality
- **User Story 2 (P2)**: Can start after US1 - Builds on ONNX integration
- **User Story 3 (P3)**: Can start after US1 - Requires ONNX working for bundle changes

### Within Each User Story

- Backend changes before frontend integration
- Core implementation before error handling
- Story complete before moving to next priority

### Parallel Opportunities

- T003, T004 can run in parallel (Setup phase)
- T010, T011, T012 can run in parallel (different files)
- T028, T029, T032 can run in parallel (Polish phase)

---

## Parallel Example: User Story 1

```bash
# Launch frontend tasks in parallel (different files):
Task: "Create useOnnxSettings.ts composable in src/composables/useOnnxSettings.ts"
Task: "Add ONNX mode toggle to SettingsView.vue in src/components/SettingsView.vue"
Task: "Add i18n strings for ONNX settings in src/i18n/"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (verify existing infrastructure)
2. Complete Phase 2: Foundational (add progress reporting)
3. Complete Phase 3: User Story 1 (frontend integration)
4. **STOP and VALIDATE**: Test ONNX extraction end-to-end
5. Deploy/demo if ready

### Incremental Delivery

1. Complete Setup + Foundational → Backend ready
2. Add User Story 1 → Test extraction → Deploy/Demo (MVP!)
3. Add User Story 2 → Test startup time → Deploy/Demo
4. Add User Story 3 → Test bundle size → Deploy/Demo
5. Each story adds value without breaking previous stories

---

## Notes

- Core ONNX inference engine already exists in `src-tauri/src/onnx_demucs.rs`
- Tauri command `extract_bgm_onnx` already registered in `src-tauri/src/lib.rs`
- Main work is frontend integration and production bundling
- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Commit after each task or logical group
