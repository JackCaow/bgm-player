//! Test ONNX inference
//!
//! Run with:
//! ORT_DYLIB_PATH=/Users/cver/miniconda3/lib/python3.13/site-packages/onnxruntime/capi/libonnxruntime.1.23.2.dylib cargo run --example test_onnx

use std::path::{Path, PathBuf};

#[derive(Debug)]
struct Args {
    input: String,
    model: String,
    seconds: Option<f32>,
    save: bool,
}

fn parse_args() -> Args {
    let mut input: Option<String> = None;
    let mut model: Option<String> = None;
    let mut seconds: Option<f32> = None;
    let mut save = false;

    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--input" => {
                if let Some(v) = iter.next() {
                    input = Some(v);
                }
            }
            "--model" => {
                if let Some(v) = iter.next() {
                    model = Some(v);
                }
            }
            "--seconds" => {
                if let Some(v) = iter.next() {
                    seconds = v.parse::<f32>().ok();
                }
            }
            "--save" => {
                save = true;
            }
            _ => {
                if !arg.starts_with('-') && input.is_none() {
                    input = Some(arg);
                }
            }
        }
    }

    Args {
        input: input.unwrap_or_else(|| "/Users/cver/Downloads/七里香_周杰伦 - 搁浅.mp3".to_string()),
        model: model.unwrap_or_else(|| "htdemucs".to_string()),
        seconds,
        save,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args();
    println!("Testing ONNX inference with full spectral processing...");

    // Initialize ONNX Runtime
    let dylib_path = std::env::var("ORT_DYLIB_PATH")
        .unwrap_or_else(|_| "./binaries/libonnxruntime-aarch64-apple-darwin.dylib".to_string());

    println!("Using ONNX Runtime: {}", dylib_path);

    ort::init_from(&dylib_path)?.commit();

    // Load model
    let base_dir = Path::new("../onnx_models");
    let model_path = base_dir.join(format!("{}_core.onnx", args.model));
    let model_path = if model_path.exists() {
        model_path
    } else {
        base_dir.join(format!("{}_core_0.onnx", args.model))
    };
    if !model_path.exists() {
        return Err(format!("Model not found: {:?}", model_path).into());
    }

    println!("Loading model: {:?}", model_path);

    #[derive(Debug, serde::Deserialize)]
    struct ModelConfigFile {
        sample_rate: u32,
        channels: usize,
        sources: Vec<String>,
        nfft: usize,
        hop_length: usize,
        #[serde(default)]
        ensemble: Option<EnsembleConfig>,
    }

    #[derive(Debug, serde::Deserialize)]
    struct EnsembleConfig {
        models: Vec<String>,
        weights: Vec<Vec<f32>>,
    }

    let config_path = base_dir.join(format!("{}_config.json", args.model));
    let (config, ensemble_models, ensemble_weights) = if config_path.exists() {
        let bytes = std::fs::read(config_path)?;
        let parsed: ModelConfigFile = serde_json::from_slice(&bytes)?;

        let segment_samples = ((parsed.sample_rate as f32) * 7.8).round() as usize;

        let ensemble_models = parsed.ensemble.as_ref().map(|e| e.models.clone());
        let ensemble_weights = parsed.ensemble.map(|e| e.weights);

        (
            get_bgm_lib::DemucsConfig {
                sample_rate: parsed.sample_rate,
                channels: parsed.channels,
                sources: parsed.sources,
                nfft: parsed.nfft,
                hop_length: parsed.hop_length,
                segment_samples,
                shifts: 1,
                execution_provider: get_bgm_lib::ExecutionProvider::Cpu,
            },
            ensemble_models,
            ensemble_weights,
        )
    } else {
        eprintln!("Warning: config not found at {:?}, using default config", config_path);
        (get_bgm_lib::DemucsConfig::default(), None, None)
    };

    let sample_rate = config.sample_rate;
    let source_names = config.sources.clone();
    println!("Sources: {:?}", source_names);

    let model_dir = model_path.parent().unwrap_or(base_dir);
    let model_paths: Vec<PathBuf> = if let Some(files) = ensemble_models.as_ref() {
        files.iter().map(|f| model_dir.join(f)).collect()
    } else {
        vec![model_path.to_path_buf()]
    };

    let mut demucs = get_bgm_lib::OnnxDemucs::new(model_paths, config, ensemble_weights)?;

    println!("Model loaded successfully!");

    // Test with a simple audio file if available
    let test_audio = Path::new(&args.input);
    if test_audio.exists() {
        println!("Testing with audio file: {:?}", test_audio);

        let t0 = std::time::Instant::now();
        let audio = get_bgm_lib::read_audio(test_audio, sample_rate)?;
        let t_read = t0.elapsed();
        println!("Audio shape: {:?}", audio.shape());

        let full_duration_sec = audio.shape()[1] as f32 / sample_rate as f32;
        println!("Full duration: {:.1}s ({:.2} min)", full_duration_sec, full_duration_sec / 60.0);

        let audio = if let Some(seconds) = args.seconds {
            let max_samples = (sample_rate as f32 * seconds).round() as usize;
            if audio.shape()[1] > max_samples {
                audio.slice(ndarray::s![.., ..max_samples]).to_owned()
            } else {
                audio
            }
        } else {
            audio
        };

        let duration_sec = audio.shape()[1] as f32 / sample_rate as f32;
        println!("Processing {} samples ({:.1}s)...", audio.shape()[1], duration_sec);

        let start = std::time::Instant::now();
        let sources = demucs.separate(&audio, None, None)?;
        let elapsed = start.elapsed();

        println!("Separation complete in {:.2}s! Got {} sources", elapsed.as_secs_f32(), sources.len());
        println!("Speed: {:.2}x realtime", duration_sec / elapsed.as_secs_f32());
        println!("Audio read time: {:.2}s", t_read.as_secs_f32());

        if args.save {
            // Save output files
            let output_dir = Path::new("../output_rust_onnx_v2").join(&args.model);
            std::fs::create_dir_all(&output_dir)?;

            let save_start = std::time::Instant::now();
            for (i, source) in sources.iter().enumerate() {
                let name = &source_names[i];
                let output_path = output_dir.join(format!("{}.wav", name));
                println!("  Saving {}: shape {:?}", name, source.shape());
                get_bgm_lib::write_wav(&output_path, source, sample_rate)?;
            }
            println!("Output saved to: {:?} ({:.2}s)", output_dir, save_start.elapsed().as_secs_f32());
        }

    } else {
        println!("No test audio file found, skipping audio test");
    }

    println!("Test complete!");
    Ok(())
}
