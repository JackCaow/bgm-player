# Repository Guidelines

## Project Structure & Module Organization
- `src/` contains the Vue 3 + TypeScript UI. Components live in `src/components/`, shared logic in `src/composables/`, helpers in `src/utils/`, translations in `src/i18n/`, and global styles in `src/style.css`.
- `src-tauri/` is the Rust backend for the desktop app. Tauri commands are in `src-tauri/src/lib.rs`, the entrypoint is `src-tauri/src/main.rs`, config is `src-tauri/tauri.conf.json`, and app icons are in `src-tauri/icons/`.
- `script/` holds Python tooling for BGM extraction (Demucs-based), used as a dev fallback when a bundled binary is unavailable.
- Generated outputs are in `dist/`, `build/`, `src-tauri/target/`, and `output/` and should not be edited manually.

## Build, Test, and Development Commands
- `npm run dev` starts the Vite dev server for the web UI.
- `npm run build` runs `vue-tsc --noEmit` and builds production assets.
- `npm run preview` serves the built web assets locally.
- `npm run tauri:dev` runs the desktop app in development mode.
- `npm run tauri:build` builds native desktop bundles.
- Example CLI extraction (optional): `python3 script/bgm_extractor.py input.wav -o output -m htdemucs`.

## Coding Style & Naming Conventions
- Match existing formatting: 2-space indentation in `.vue` templates/styles and double quotes in TS/JS.
- Vue components are PascalCase filenames (e.g., `AppSidebar.vue`); composables are `useX.ts` in `src/composables/`.
- There is no configured linter/formatter; keep changes consistent with nearby code.

## Testing Guidelines
- No automated test framework is configured in this repository.
- Use `npm run build` for type-checking and do manual UI smoke tests (queue, result, merge, history flows).
- If you add tests, document the framework and add an npm script for running them.

## Commit & Pull Request Guidelines
- This checkout does not include a `.git` directory, so commit conventions cannot be inferred.
- Until a convention is established, use short, imperative commit subjects and include a brief scope if helpful.
- PRs should include a concise summary, testing notes, and screenshots for UI changes.

## Environment & Dependencies
- `ffmpeg` must be available on PATH for track merging in the Tauri backend.
- The Python fallback expects `python3` plus Demucs/Torch/NumPy/SciPy installed.
