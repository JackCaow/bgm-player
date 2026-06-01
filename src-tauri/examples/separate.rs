//! Parity helper: run the Rust+ONNX htdemucs 2-track separator on a single
//! input file and write `vocals.wav` / `no_vocals.wav` into an output dir.
//!
//! Usage (from repo root, so the model dev-fallback resolves):
//!   cargo run --example separate -- <input.wav> <output_dir> [ffmpeg]
//!
//! Output layout matches the app: `<output_dir>/htdemucs/<input_stem>/{vocals,no_vocals}.wav`.
//! Used by Task 8 parity verification against the Python/PyTorch path.

use std::path::Path;

fn main() {
    let mut args = std::env::args().skip(1);
    let input = args.next().expect("usage: separate <input.wav> <output_dir> [ffmpeg]");
    let output = args.next().expect("usage: separate <input.wav> <output_dir> [ffmpeg]");
    let ffmpeg = args.next().unwrap_or_else(|| "ffmpeg".to_string());

    // resource_dir: pass `src-tauri` so the production path
    // `<resource_dir>/models/htdemucs.onnx` resolves when run from repo root;
    // resolve_model_path also has cwd-based fallbacks.
    let resource_dir = Path::new("src-tauri");

    let t = std::time::Instant::now();
    let paths = bgm_player_lib::separator::separate_htdemucs_2track(
        resource_dir,
        Path::new(&input),
        Path::new(&output),
        &ffmpeg,
        &|p, msg| eprintln!("progress {p:.1}%: {msg}"),
    )
    .expect("separation should succeed");

    eprintln!("finished in {:?}", t.elapsed());
    println!("vocals:    {}", paths.vocals.display());
    println!("no_vocals: {}", paths.no_vocals.display());
}
