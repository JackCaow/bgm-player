//! Test ONNX inference with full song

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing ONNX inference with full song...");

    // Initialize ONNX Runtime
    let dylib_path = std::env::var("ORT_DYLIB_PATH")
        .unwrap_or_else(|_| "../binaries/libonnxruntime-aarch64-apple-darwin.dylib".to_string());

    println!("Using ONNX Runtime: {}", dylib_path);
    ort::init_from(&dylib_path)?.commit();

    // Load model
    let model_path = std::path::Path::new("../../onnx_models/htdemucs_core.onnx");
    
    println!("Loading model...");
    let config = get_bgm_lib::onnx_demucs::DemucsConfig::default();
    let source_names = config.sources.clone();
    let mut demucs = get_bgm_lib::onnx_demucs::OnnxDemucs::new(
        vec![model_path.to_path_buf()],
        config,
        None,
    )?;

    // Test with full song
    let test_audio = std::path::Path::new("/Users/cver/Downloads/周杰伦 - 稻香.mp3");
    println!("Reading audio: {:?}", test_audio);

    let audio = get_bgm_lib::onnx_demucs::read_audio(test_audio, 44100)?;
    let duration_sec = audio.shape()[1] as f32 / 44100.0;
    println!("Duration: {:.1}s ({:.1} minutes)", duration_sec, duration_sec / 60.0);

    println!("\nStarting separation...");
    let start = std::time::Instant::now();
    let sources = demucs.separate(&audio, None, None)?;
    let elapsed = start.elapsed();

    println!("\n✅ Separation complete!");
    println!("  Time: {:.1}s ({:.1} minutes)", elapsed.as_secs_f32(), elapsed.as_secs_f32() / 60.0);
    println!("  Speed: {:.2}x realtime", duration_sec / elapsed.as_secs_f32());
    
    // Save output
    let output_dir = std::path::Path::new("../../output_full_song");
    std::fs::create_dir_all(output_dir)?;

    for (i, source) in sources.iter().enumerate() {
        let output_path = output_dir.join(format!("{}.wav", source_names[i]));
        get_bgm_lib::onnx_demucs::write_wav(&output_path, source, 44100)?;
    }
    println!("Output saved to: {:?}", output_dir);

    Ok(())
}
