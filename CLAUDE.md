# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

BGM Extractor is a Tauri-based desktop application that extracts background music from songs using Demucs AI models. The app has a Vue 3 frontend and Rust backend, with Python scripts for the actual BGM extraction.

**Tech Stack**: Vue 3 + TypeScript (frontend), Rust + Tauri 2 (backend), Python + Demucs (audio processing)

## Development Commands

- `npm run dev` - Start Vite dev server (frontend only)
- `npm run tauri:dev` - Run full desktop app in development mode
- `npm run build` - Type-check and build frontend assets
- `npm run tauri:build` - Build native desktop bundles (.app, .dmg on macOS)
- `npm run preview` - Preview built frontend assets

## Architecture

### Frontend (`src/`)
- **Components** (`src/components/`): Main UI views are suffixed with `View.vue` (QueueView, ResultView, HistoryView, MergeView, SettingsView). AppSidebar handles navigation.
- **Composables** (`src/composables/`): Shared reactive logic following `useX.ts` pattern. Key composables:
  - `useFiles.ts` - File selection and queue management
  - `useHistory.ts` - Extraction history persistence
  - `useMerge.ts` - Track merging with volume control
  - `useWaveform.ts` - Audio waveform visualization
  - `useTheme.ts` - Theme switching
- **Utilities** (`src/utils/`): Helper functions
- **i18n** (`src/i18n/`): Internationalization (supports Chinese and English)
- **Styling**: Global styles in `src/style.css` (2-space indentation)

### Backend (`src-tauri/`)
- **Tauri Commands** (`src-tauri/src/lib.rs`): Rust functions exposed to frontend
  - `extract_bgm()` - Main extraction command, uses bundled binary or falls back to Python script
  - `merge_tracks()` - Combines audio files with volume adjustment using ffmpeg
  - Progress events emitted via `extraction-progress` and `merge-progress` channels
- **Binary Resolution**: App looks for bundled `bgm-extractor` binary in:
  1. Production: `Contents/MacOS/bgm-extractor`
  2. Dev mode: `binaries/bgm-extractor-{target}` (platform-specific)
  3. Fallback: Python script at `script/extract_bgm.py`
- **Configuration**: `src-tauri/tauri.conf.json` defines app metadata, window settings, and bundled binaries

### Python Scripts (`script/`)
- `bgm_extractor.py` - CLI tool for BGM extraction using Demucs models
- `extract_bgm.py` - Simplified extraction script used as fallback
- `build_binary.py` - PyInstaller script to create standalone binary
- `benchmark.py` - Performance testing for different models

**Models**: `htdemucs` (default, best quality), `htdemucs_ft`, `htdemucs_6s`, `hdemucs_mmi`

## External Dependencies

- **ffmpeg**: Required on PATH for track merging functionality
- **Python 3 + Demucs**: Only needed if running Python scripts directly (not required for bundled binary)

## Code Style

- **Indentation**: 2 spaces in `.vue` files and TS/JS
- **Quotes**: Double quotes in TypeScript/JavaScript
- **Vue Components**: PascalCase filenames (e.g., `QueueView.vue`)
- **Composables**: `useX.ts` pattern with default exports
- **No linter/formatter configured** - match existing code style

## Scripts Location

All scripts should be created in the `script/` folder per project conventions.
