use regex::Regex;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tauri::{Emitter, Manager, Window};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackInfo {
    pub track_type: String,
    pub path: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExtractResult {
    pub mode: String,
    pub tracks: Vec<TrackInfo>,
    // Backward compatibility
    pub bgm_path: Option<String>,
    pub vocals_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MergeResult {
    pub output_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackInput {
    pub path: String,
    pub volume: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProgressPayload {
    pub progress: f64,
    pub status: String,
    pub stage: String,
}

fn parse_progress(line: &str) -> Option<f64> {
    let re = Regex::new(r"^\s*(\d+)%\|").ok()?;
    if let Some(caps) = re.captures(line) {
        if let Some(m) = caps.get(1) {
            return m.as_str().parse::<f64>().ok();
        }
    }
    None
}

// Helper function to get bundled ffmpeg path
fn get_ffmpeg_path(app: &tauri::AppHandle) -> String {
    // Try to find bundled ffmpeg
    if let Ok(resource_dir) = app.path().resource_dir() {
        // Production: resource_dir is Contents/Resources, binary is in Contents/MacOS
        if let Some(parent) = resource_dir.parent() {
            let macos_path = parent.join("MacOS").join("ffmpeg");
            if macos_path.exists() {
                return macos_path.to_string_lossy().to_string();
            }
        }
    }

    // Dev mode: try binaries folder with platform suffix
    let target = if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "aarch64-apple-darwin"
        } else {
            "x86_64-apple-darwin"
        }
    } else if cfg!(target_os = "windows") {
        "x86_64-pc-windows-msvc.exe"
    } else {
        "x86_64-unknown-linux-gnu"
    };

    if let Ok(cwd) = std::env::current_dir() {
        let dev_path = cwd.join("binaries").join(format!("ffmpeg-{}", target));
        if dev_path.exists() {
            return dev_path.to_string_lossy().to_string();
        }
    }

    // Fallback to system ffmpeg
    "ffmpeg".to_string()
}

// Helper function to get track display name
fn get_track_name(track_type: &str) -> String {
    match track_type {
        "vocals" => "人声".to_string(),
        "drums" => "鼓".to_string(),
        "bass" => "贝斯".to_string(),
        "other" => "其他".to_string(),
        "guitar" => "吉他".to_string(),
        "piano" => "钢琴".to_string(),
        "no_vocals" => "BGM".to_string(),
        _ => track_type.to_string(),
    }
}

#[tauri::command]
async fn extract_bgm(
    app: tauri::AppHandle,
    window: Window,
    input: String,
    output: Option<String>,
    model: String,
    format: Option<String>,
    _quality: Option<String>,
    bitrate: Option<u32>,
    sample_rate: Option<u32>,
    use_gpu: Option<bool>,
    separation_mode: Option<String>,
) -> Result<ExtractResult, String> {
    let input_path = PathBuf::from(&input);

    if !input_path.exists() {
        return Err(format!("文件不存在: {}", input));
    }

    let _ = window.emit(
        "extraction-progress",
        ProgressPayload {
            progress: 0.0,
            status: "正在初始化...".to_string(),
            stage: "init".to_string(),
        },
    );

    // Debug: log separation_mode at the start
    eprintln!("[DEBUG] extract_bgm called with separation_mode: {:?}", separation_mode);

    // Try to find the bundled binary
    // In production: Contents/MacOS/bgm-extractor
    // In dev: src-tauri/binaries/bgm-extractor-{target}
    let binary_path = app
        .path()
        .resource_dir()
        .ok()
        .and_then(|p| {
            // Production: resource_dir is Contents/Resources, binary is in Contents/MacOS
            let macos_path = p.parent()?.join("MacOS").join("bgm-extractor");
            if macos_path.exists() {
                return Some(macos_path);
            }
            None
        })
        .or_else(|| {
            // Dev mode: try binaries folder with platform suffix
            let target = if cfg!(target_os = "macos") {
                if cfg!(target_arch = "aarch64") {
                    "aarch64-apple-darwin"
                } else {
                    "x86_64-apple-darwin"
                }
            } else if cfg!(target_os = "windows") {
                "x86_64-pc-windows-msvc"
            } else {
                "x86_64-unknown-linux-gnu"
            };

            let dev_path = std::env::current_dir()
                .ok()?
                .join("binaries")
                .join(format!("bgm-extractor-{}", target));

            if dev_path.exists() {
                return Some(dev_path);
            }

            // Also try parent directory (if running from src-tauri)
            let parent_path = std::env::current_dir()
                .ok()?
                .parent()?
                .join("src-tauri")
                .join("binaries")
                .join(format!("bgm-extractor-{}", target));

            if parent_path.exists() {
                return Some(parent_path);
            }

            None
        });

    // Get output directory
    let output_dir = if let Some(ref out) = output {
        PathBuf::from(out)
    } else {
        // Default output directory
        dirs::document_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
            .join("BGM Extractor Output")
    };

    let _ = window.emit(
        "extraction-progress",
        ProgressPayload {
            progress: 5.0,
            status: "正在启动音频分离引擎...".to_string(),
            stage: "starting".to_string(),
        },
    );

    // Build and run command
    let mut child = if let Some(bin_path) = binary_path {
        let mut cmd = Command::new(&bin_path);

        // Set PATH to include the directory containing ffmpeg/ffprobe
        if let Some(bin_dir) = bin_path.parent() {
            let current_path = std::env::var("PATH").unwrap_or_default();
            let new_path = format!("{}:{}", bin_dir.to_string_lossy(), current_path);
            cmd.env("PATH", new_path);
        }

        cmd.arg(&input)
            .arg("-o")
            .arg(&output_dir)
            .arg("-m")
            .arg(&model);

        // Add two-stems flag only for 2-track mode
        let mode = separation_mode.as_deref().unwrap_or("2-track");
        if mode == "2-track" {
            cmd.arg("--two-stems").arg("vocals");
        }

        // Add GPU flag if enabled (use MPS on macOS, CUDA on others)
        if use_gpu.unwrap_or(false) {
            if cfg!(target_os = "macos") {
                cmd.arg("--device").arg("mps");
            } else {
                cmd.arg("--device").arg("cuda");
            }
        }

        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("执行程序失败: {} (路径: {:?})", e, bin_path))?
    } else {
        // Fallback to Python script (dev mode only)
        let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
        let project_root = if current_dir.ends_with("src-tauri") {
            current_dir.parent().unwrap().to_path_buf()
        } else {
            current_dir
        };

        let script_path = project_root.join("script").join("bgm_extractor.py");
        if !script_path.exists() {
            return Err(format!("脚本不存在: {:?}", script_path));
        }

        let home = std::env::var("HOME").unwrap_or_default();
        let conda_path = format!("{}/miniconda3/bin", home);
        let current_path = std::env::var("PATH").unwrap_or_default();
        let new_path = format!("{}:{}", conda_path, current_path);

        // Debug log
        eprintln!("[DEBUG] script_path: {:?}", script_path);
        eprintln!("[DEBUG] input: {:?}", input);
        eprintln!("[DEBUG] output_dir: {:?}", output_dir);
        eprintln!("[DEBUG] model: {:?}", model);
        eprintln!("[DEBUG] separation_mode: {:?}", separation_mode);

        let mut cmd = Command::new("python3");
        cmd.arg(&script_path)
            .arg(&input)
            .arg("-o")
            .arg(&output_dir)
            .arg("-m")
            .arg(&model);

        // Add two-stems flag only for 2-track mode
        let mode = separation_mode.as_deref().unwrap_or("2-track");
        if mode == "2-track" {
            cmd.arg("--two-stems").arg("vocals");
        }

        // Add GPU flag if enabled (use MPS on macOS, CUDA on others)
        if use_gpu.unwrap_or(false) {
            if cfg!(target_os = "macos") {
                cmd.arg("--device").arg("mps");
            } else {
                cmd.arg("--device").arg("cuda");
            }
        }

        cmd.env("PATH", &new_path)
            .env("PYTHONUNBUFFERED", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("执行脚本失败: {}", e))?
    };

    let stderr = child.stderr.take().ok_or("无法获取 stderr")?;
    let reader = BufReader::new(stderr);

    let _ = window.emit(
        "extraction-progress",
        ProgressPayload {
            progress: 10.0,
            status: "正在分离音轨...".to_string(),
            stage: "processing".to_string(),
        },
    );

    let mut stderr_output = Vec::new();
    let mut max_progress: f64 = 0.0;  // 跟踪最大进度，防止回退
    for line in reader.lines() {
        if let Ok(line) = line {
            stderr_output.push(line.clone());
            if let Some(pct) = parse_progress(&line) {
                // 只有当新进度大于当前最大进度时才更新
                if pct > max_progress {
                    max_progress = pct;
                    let mapped_progress = 10.0 + (pct * 0.85);
                    let _ = window.emit(
                        "extraction-progress",
                        ProgressPayload {
                            progress: mapped_progress,
                            status: format!("正在分离音轨... {}%", pct as i32),
                            stage: "processing".to_string(),
                        },
                    );
                }
            }
        }
    }

    let status = child.wait().map_err(|e| format!("等待进程失败: {}", e))?;

    if !status.success() {
        let error_msg = stderr_output.iter()
            .filter(|l| l.contains("错误") || l.contains("Error") || l.contains("error") || l.contains("Traceback") || l.contains("Exception"))
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        if error_msg.is_empty() {
            return Err(format!("处理失败 (退出码: {:?})", status.code()));
        } else {
            return Err(format!("处理失败: {}", error_msg));
        }
    }

    let _ = window.emit(
        "extraction-progress",
        ProgressPayload {
            progress: 98.0,
            status: "正在保存文件...".to_string(),
            stage: "saving".to_string(),
        },
    );

    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("无法获取文件名")?;

    let result_dir = output_dir.join(&model).join(stem);
    let mode = separation_mode.as_deref().unwrap_or("2-track");

    // Build track list based on separation mode and model
    // htdemucs_6s outputs 6 tracks, others output 4 tracks
    let mut tracks: Vec<TrackInfo> = Vec::new();
    let track_types: Vec<&str> = match mode {
        "2-track" => vec!["vocals", "no_vocals"],
        "4-track" => vec!["vocals", "drums", "bass", "other"],
        "6-track" => {
            // Only htdemucs_6s supports 6 tracks
            if model == "htdemucs_6s" {
                vec!["vocals", "drums", "bass", "guitar", "piano", "other"]
            } else {
                // Fallback to 4-track for other models
                vec!["vocals", "drums", "bass", "other"]
            }
        }
        _ => return Err(format!("Invalid separation mode: {}", mode)),
    };

    // Check if files exist and build track info
    for track_type in &track_types {
        let track_path = result_dir.join(format!("{}.wav", track_type));
        if !track_path.exists() {
            return Err(format!("输出文件不存在: {:?}", track_path));
        }
        tracks.push(TrackInfo {
            track_type: track_type.to_string(),
            path: track_path.to_string_lossy().to_string(),
            name: get_track_name(track_type),
        });
    }

    // Convert format if requested and not WAV
    let final_format = format.as_deref().unwrap_or("wav");
    if final_format != "wav" {
        let _ = window.emit(
            "extraction-progress",
            ProgressPayload {
                progress: 98.0,
                status: format!("正在转换为 {} 格式...", final_format.to_uppercase()),
                stage: "converting".to_string(),
            },
        );

        // Convert all tracks to the requested format
        for track in &mut tracks {
            let track_path = PathBuf::from(&track.path);
            let converted_path = convert_audio_format(
                &app,
                &track_path,
                final_format,
                bitrate,
                sample_rate,
            )?;
            track.path = converted_path.to_string_lossy().to_string();
        }
    }

    let _ = window.emit(
        "extraction-progress",
        ProgressPayload {
            progress: 100.0,
            status: "处理完成！".to_string(),
            stage: "done".to_string(),
        },
    );

    // Backward compatibility: set bgm_path and vocals_path for 2-track mode
    let (bgm_path, vocals_path) = if mode == "2-track" {
        let vocals = tracks.iter().find(|t| t.track_type == "vocals");
        let bgm = tracks.iter().find(|t| t.track_type == "no_vocals");
        (
            bgm.map(|t| t.path.clone()),
            vocals.map(|t| t.path.clone()),
        )
    } else {
        (None, None)
    };

    Ok(ExtractResult {
        mode: mode.to_string(),
        tracks,
        bgm_path,
        vocals_path,
    })
}

// Helper function to convert audio format using ffmpeg
fn convert_audio_format(
    app: &tauri::AppHandle,
    input_path: &PathBuf,
    format: &str,
    bitrate: Option<u32>,
    sample_rate: Option<u32>,
) -> Result<PathBuf, String> {
    let output_path = input_path.with_extension(format);
    let ffmpeg_path = get_ffmpeg_path(app);

    // Debug log
    eprintln!("[DEBUG] convert_audio_format: input={:?}, output={:?}, ffmpeg={}", input_path, output_path, ffmpeg_path);

    // Check if input file exists
    if !input_path.exists() {
        return Err(format!("Input file does not exist: {:?}", input_path));
    }

    let mut cmd = Command::new(&ffmpeg_path);
    cmd.arg("-y") // Overwrite output
        .arg("-i")
        .arg(input_path);

    // Set sample rate if specified
    if let Some(sr) = sample_rate {
        cmd.arg("-ar").arg(sr.to_string());
    }

    // Set bitrate for lossy formats
    match format {
        "mp3" => {
            cmd.arg("-codec:a").arg("libmp3lame");
            if let Some(br) = bitrate {
                cmd.arg("-b:a").arg(format!("{}k", br));
            }
        }
        "aac" => {
            cmd.arg("-codec:a").arg("aac");
            if let Some(br) = bitrate {
                cmd.arg("-b:a").arg(format!("{}k", br));
            }
        }
        "m4a" => {
            cmd.arg("-codec:a").arg("aac");
            if let Some(br) = bitrate {
                cmd.arg("-b:a").arg(format!("{}k", br));
            }
        }
        "ogg" => {
            cmd.arg("-codec:a").arg("libvorbis");
            if let Some(br) = bitrate {
                cmd.arg("-b:a").arg(format!("{}k", br));
            } else {
                // Default to quality-based encoding (q:a 6 = ~192kbps)
                cmd.arg("-q:a").arg("6");
            }
        }
        "opus" => {
            cmd.arg("-codec:a").arg("libopus");
            if let Some(br) = bitrate {
                cmd.arg("-b:a").arg(format!("{}k", br));
            } else {
                // Default to 128kbps for Opus (good quality)
                cmd.arg("-b:a").arg("128k");
            }
        }
        "flac" => {
            cmd.arg("-codec:a").arg("flac");
        }
        "wav" => {
            cmd.arg("-codec:a").arg("pcm_s16le");
        }
        _ => {
            return Err(format!("Unsupported format: {}", format));
        }
    }

    cmd.arg(&output_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    eprintln!("[DEBUG] Running ffmpeg command...");

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run ffmpeg: {}. Make sure ffmpeg is installed.", e))?;

    eprintln!("[DEBUG] ffmpeg exit status: {:?}", output.status);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        eprintln!("[DEBUG] ffmpeg stderr length: {}", stderr.len());
        eprintln!("[DEBUG] ffmpeg stdout length: {}", stdout.len());
        return Err(format!("Failed to convert to {} format (exit code: {:?})", format, output.status.code()));
    }

    eprintln!("[DEBUG] ffmpeg conversion successful");
    Ok(output_path)
}

#[tauri::command]
async fn merge_tracks(
    app: tauri::AppHandle,
    bgm_path: String,
    vocals_path: String,
    bgm_volume: f64,
    vocals_volume: f64,
    output_path: Option<String>,
) -> Result<MergeResult, String> {
    let bgm_file = PathBuf::from(&bgm_path);
    let vocals_file = PathBuf::from(&vocals_path);

    if !bgm_file.exists() {
        return Err(format!("BGM file not found: {}", bgm_path));
    }
    if !vocals_file.exists() {
        return Err(format!("Vocals file not found: {}", vocals_path));
    }

    // Determine output path
    let output = if let Some(out) = output_path {
        PathBuf::from(out)
    } else {
        let parent = bgm_file.parent().unwrap_or(std::path::Path::new("."));
        let stem = bgm_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("merged");
        parent.join(format!("{}_merged.wav", stem.replace("no_vocals", "mixed")))
    };

    // Use ffmpeg to merge tracks with volume adjustment
    // Filter: adjust volume of each input, then mix them together
    let filter = format!(
        "[0:a]volume={}[a];[1:a]volume={}[b];[a][b]amix=inputs=2:duration=longest[out]",
        bgm_volume, vocals_volume
    );

    let ffmpeg_path = get_ffmpeg_path(&app);
    let status = Command::new(&ffmpeg_path)
        .arg("-y") // Overwrite output
        .arg("-i")
        .arg(&bgm_path)
        .arg("-i")
        .arg(&vocals_path)
        .arg("-filter_complex")
        .arg(&filter)
        .arg("-map")
        .arg("[out]")
        .arg("-ac")
        .arg("2") // Stereo output
        .arg(&output)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("Failed to run ffmpeg: {}. Make sure ffmpeg is installed.", e))?;

    if !status.success() {
        return Err("Failed to merge tracks. Please check the input files.".to_string());
    }

    Ok(MergeResult {
        output_path: output.to_string_lossy().to_string(),
    })
}

#[tauri::command]
async fn merge_multi_tracks(
    app: tauri::AppHandle,
    tracks: Vec<TrackInput>,
    output_path: Option<String>,
) -> Result<MergeResult, String> {
    if tracks.len() < 2 {
        return Err("At least 2 tracks are required for merging".to_string());
    }

    // Validate all files exist
    for (i, track) in tracks.iter().enumerate() {
        let path = PathBuf::from(&track.path);
        if !path.exists() {
            return Err(format!("Track {} file not found: {}", i + 1, track.path));
        }
    }

    // Determine output path
    let first_file = PathBuf::from(&tracks[0].path);
    let output = if let Some(out) = output_path {
        PathBuf::from(out)
    } else {
        let parent = first_file.parent().unwrap_or(std::path::Path::new("."));
        let stem = first_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("merged");
        parent.join(format!("{}_merged_{}.wav", stem, tracks.len()))
    };

    // Build ffmpeg filter for multiple tracks
    // Example for 3 tracks: [0:a]volume=1.0[a0];[1:a]volume=0.8[a1];[2:a]volume=0.5[a2];[a0][a1][a2]amix=inputs=3:duration=longest[out]
    let mut filter_parts: Vec<String> = Vec::new();
    let mut mix_inputs: Vec<String> = Vec::new();

    for (i, track) in tracks.iter().enumerate() {
        filter_parts.push(format!("[{}:a]volume={}[a{}]", i, track.volume, i));
        mix_inputs.push(format!("[a{}]", i));
    }

    let filter = format!(
        "{};{}amix=inputs={}:duration=longest[out]",
        filter_parts.join(";"),
        mix_inputs.join(""),
        tracks.len()
    );

    // Build ffmpeg command with all input files
    let ffmpeg_path = get_ffmpeg_path(&app);
    let mut cmd = Command::new(&ffmpeg_path);
    cmd.arg("-y"); // Overwrite output

    for track in &tracks {
        cmd.arg("-i").arg(&track.path);
    }

    let status = cmd
        .arg("-filter_complex")
        .arg(&filter)
        .arg("-map")
        .arg("[out]")
        .arg("-ac")
        .arg("2") // Stereo output
        .arg(&output)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("Failed to run ffmpeg: {}. Make sure ffmpeg is installed.", e))?;

    if !status.success() {
        return Err("Failed to merge tracks. Please check the input files.".to_string());
    }

    Ok(MergeResult {
        output_path: output.to_string_lossy().to_string(),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![extract_bgm, merge_tracks, merge_multi_tracks])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
