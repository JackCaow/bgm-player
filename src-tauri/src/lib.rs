use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Manager, Window};
use kokorox::tts::koko::TTSKoko;

mod onnx_demucs;
pub use onnx_demucs::{
    read_audio, write_wav, DemucsConfig, ExecutionProvider, OnnxDemucs, ProgressCallback,
};

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PodcastRole {
    pub name: String,
    pub voice: String,
    pub persona: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PodcastLine {
    pub role: String,
    pub voice: String,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PodcastOutlineSection {
    pub title: String,
    pub objective: String,
    pub target_seconds: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PodcastResult {
    pub output_path: String,
    pub script_path: String,
    pub lines: Vec<PodcastLine>,
    pub outline: Vec<PodcastOutlineSection>,
    pub actual_duration_seconds: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PodcastScriptPreviewResult {
    pub script_path: String,
    pub lines: Vec<PodcastLine>,
    pub outline: Vec<PodcastOutlineSection>,
    pub estimated_duration_seconds: Option<f64>,
    pub target_duration_seconds: Option<u32>,
    pub style: String,
}

static ORT_INIT: OnceLock<Result<(), String>> = OnceLock::new();
static KOKORO_TTS_CACHE: Mutex<Option<TTSKoko>> = Mutex::new(None);
const KOKORO_MODEL_URL: &str = "https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/kokoro-v1.0.onnx";
const KOKORO_VOICES_URL: &str = "https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/voices-v1.0.bin";
fn log_stderr(message: &str) {
    let _ = std::io::stderr().write_all(message.as_bytes());
    let _ = std::io::stderr().write_all(b"\n");
}

fn emit_podcast_progress(window: &Window, progress: f64, stage: &str, status: impl Into<String>) {
    let p = if progress.is_finite() {
        progress.clamp(0.0, 100.0)
    } else {
        0.0
    };
    let _ = window.emit(
        "podcast-progress",
        ProgressPayload {
            progress: p,
            stage: stage.to_string(),
            status: status.into(),
        },
    );
}

fn get_ffprobe_path(app: &tauri::AppHandle) -> String {
    // Try to find bundled ffprobe
    if let Ok(resource_dir) = app.path().resource_dir() {
        if let Some(parent) = resource_dir.parent() {
            let macos_path = parent.join("MacOS").join("ffprobe");
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
        let dev_path = cwd.join("binaries").join(format!("ffprobe-{}", target));
        if dev_path.exists() {
            return dev_path.to_string_lossy().to_string();
        }
    }

    // Fallback to system ffprobe
    "ffprobe".to_string()
}

#[tauri::command]
async fn probe_audio_duration(app: tauri::AppHandle, input: String) -> Result<f64, String> {
    let input_path = PathBuf::from(&input);
    if !input_path.exists() {
        return Err(format!("文件不存在: {}", input));
    }

    let ffprobe_path = get_ffprobe_path(&app);
    let output = Command::new(&ffprobe_path)
        .arg("-v")
        .arg("error")
        .arg("-show_entries")
        .arg("format=duration")
        .arg("-of")
        .arg("default=noprint_wrappers=1:nokey=1")
        .arg(&input)
        .output()
        .map_err(|e| format!("Failed to run ffprobe: {}. Make sure ffprobe is installed.", e))?;

    if !output.status.success() {
        return Err("无法读取音频时长，请检查文件格式。".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let duration = stdout.trim().parse::<f64>().map_err(|_| {
        format!(
            "无法解析 ffprobe 输出: {}",
            stdout.trim().chars().take(120).collect::<String>()
        )
    })?;

    Ok(duration)
}

fn probe_media_duration_seconds(ffprobe_path: &str, input: &str) -> Result<f64, String> {
    let output = Command::new(ffprobe_path)
        .arg("-v")
        .arg("error")
        .arg("-show_entries")
        .arg("format=duration")
        .arg("-of")
        .arg("default=noprint_wrappers=1:nokey=1")
        .arg(input)
        .output()
        .map_err(|e| format!("Failed to run ffprobe: {}. Make sure ffprobe is installed.", e))?;

    if !output.status.success() {
        return Err("无法读取媒体时长，请检查文件格式。".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let duration = stdout.trim().parse::<f64>().map_err(|_| {
        format!(
            "无法解析 ffprobe 输出: {}",
            stdout.trim().chars().take(120).collect::<String>()
        )
    })?;

    if !duration.is_finite() || duration <= 0.0 {
        return Err("无法读取媒体时长，请检查文件格式。".to_string());
    }

    Ok(duration)
}

fn probe_has_audio_stream(ffprobe_path: &str, input: &str) -> Result<bool, String> {
    let output = Command::new(ffprobe_path)
        .arg("-v")
        .arg("error")
        .arg("-select_streams")
        .arg("a:0")
        .arg("-show_entries")
        .arg("stream=index")
        .arg("-of")
        .arg("csv=p=0")
        .arg(input)
        .output()
        .map_err(|e| format!("Failed to run ffprobe: {}. Make sure ffprobe is installed.", e))?;

    if !output.status.success() {
        return Err("无法读取视频音轨信息，请检查文件格式。".to_string());
    }

    Ok(!String::from_utf8_lossy(&output.stdout).trim().is_empty())
}

#[cfg(not(debug_assertions))]
fn maybe_navigate_to_dev_url(app: &tauri::AppHandle) {
    let dev_url = match std::env::var("BGM_DEV_URL") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => return,
    };

    let window: tauri::WebviewWindow = match app.get_webview_window("main") {
        Some(w) => w,
        None => {
            log_stderr("[BGM_DEV_URL] main webview window not found");
            return;
        }
    };

    let url_literal = match serde_json::to_string(&dev_url) {
        Ok(v) => v,
        Err(e) => {
            log_stderr(&format!("[BGM_DEV_URL] failed to encode URL: {}", e));
            return;
        }
    };

    // Use JS navigation so we can keep the default window config intact.
    let js = format!("window.location.replace({});", url_literal);
    if let Err(e) = window.eval(&js) {
        log_stderr(&format!("[BGM_DEV_URL] failed to navigate: {}", e));
    } else {
        log_stderr(&format!("[BGM_DEV_URL] navigated to {}", dev_url));
    }
}

fn ort_dylib_suffix() -> &'static str {
    if cfg!(target_os = "macos") {
        "dylib"
    } else if cfg!(target_os = "windows") {
        "dll"
    } else {
        "so"
    }
}

fn ort_target_triple() -> &'static str {
    if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "aarch64-apple-darwin"
        } else {
            "x86_64-apple-darwin"
        }
    } else if cfg!(target_os = "windows") {
        "x86_64-pc-windows-msvc"
    } else {
        "x86_64-unknown-linux-gnu"
    }
}

fn get_onnxruntime_dylib_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    let target = ort_target_triple();
    let suffix = ort_dylib_suffix();
    let filename = format!("libonnxruntime-{}.{}", target, suffix);

    // Production/dev: bundle resources
    if let Ok(resource_dir) = app.path().resource_dir() {
        let candidate = resource_dir.join("binaries").join(&filename);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    // Dev mode: try local binaries folder
    if let Ok(cwd) = std::env::current_dir() {
        let candidate = cwd.join("src-tauri").join("binaries").join(&filename);
        if candidate.exists() {
            return Some(candidate);
        }
        let candidate = cwd.join("binaries").join(&filename);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}

fn ensure_ort_initialized(app: &tauri::AppHandle) -> Result<(), String> {
    match ORT_INIT.get_or_init(|| {
        let dylib_path = get_onnxruntime_dylib_path(app)
            .ok_or_else(|| "找不到 ONNX Runtime 动态库（libonnxruntime）".to_string())?;

        ort::init_from(dylib_path.to_string_lossy().as_ref())
            .map_err(|e| format!("初始化 ONNX Runtime 失败: {}", e))?
            .commit();

        Ok(())
    }) {
        Ok(()) => Ok(()),
        Err(e) => Err(e.clone()),
    }
}

fn get_onnx_model_filename(model_key: &str) -> Option<&'static str> {
    match model_key {
        "htdemucs" => Some("htdemucs_core.onnx"),
        "htdemucs_ft" => Some("htdemucs_ft_core_0.onnx"),
        "htdemucs_6s" => Some("htdemucs_6s_core.onnx"),
        _ => None,
    }
}

fn get_onnx_config_filename(model_key: &str) -> Option<&'static str> {
    match model_key {
        "htdemucs" => Some("htdemucs_config.json"),
        "htdemucs_ft" => Some("htdemucs_ft_config.json"),
        "htdemucs_6s" => Some("htdemucs_6s_config.json"),
        _ => None,
    }
}

fn get_onnx_model_path(app: &tauri::AppHandle, model_key: &str) -> Option<PathBuf> {
    let filename = get_onnx_model_filename(model_key)?;

    if let Ok(resource_dir) = app.path().resource_dir() {
        let p = resource_dir.join("onnx_models").join(filename);
        if p.exists() {
            return Some(p);
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        let p = cwd.join("onnx_models").join(filename);
        if p.exists() {
            return Some(p);
        }
        let p = cwd.join("src-tauri").join("..").join("onnx_models").join(filename);
        if p.exists() {
            return Some(p);
        }
    }

    None
}

fn get_onnx_config_path(app: &tauri::AppHandle, model_key: &str) -> Option<PathBuf> {
    let filename = get_onnx_config_filename(model_key)?;

    if let Ok(resource_dir) = app.path().resource_dir() {
        let p = resource_dir.join("onnx_models").join(filename);
        if p.exists() {
            return Some(p);
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        let p = cwd.join("onnx_models").join(filename);
        if p.exists() {
            return Some(p);
        }
        let p = cwd.join("src-tauri").join("..").join("onnx_models").join(filename);
        if p.exists() {
            return Some(p);
        }
    }

    None
}

fn find_existing_in_kokoro_dir(
    app: &tauri::AppHandle,
    names: &[&str],
    subdir: &str,
) -> Option<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();

    if let Ok(resource_dir) = app.path().resource_dir() {
        roots.push(resource_dir.join(subdir));
    }

    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd.join(subdir));
        roots.push(cwd.join("src-tauri").join("..").join(subdir));
    }

    for root in roots {
        for name in names {
            let p = root.join(name);
            if p.exists() {
                return Some(p);
            }
        }
    }

    None
}

fn get_kokoro_model_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    find_existing_in_kokoro_dir(
        app,
        &[
            "kokoro-v1.1.onnx",
            "kokoro-v1.0.onnx",
            "kokoro.onnx",
            "model.onnx",
        ],
        "tts_models/kokoro",
    )
}

fn get_kokoro_voices_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    find_existing_in_kokoro_dir(
        app,
        &[
            "voices-v1.1.bin",
            "voices-v1.0.bin",
            "voices.bin",
            "voices.json",
        ],
        "tts_models/kokoro",
    )
}

fn default_kokoro_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("bgm-extractor")
        .join("kokoro")
}

fn download_to_file(url: &str, dest: &PathBuf) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
    }

    let client = reqwest::blocking::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| format!("初始化下载客户端失败: {}", e))?;

    let response = client
        .get(url)
        .send()
        .map_err(|e| format!("下载失败: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("下载失败，HTTP 状态码: {}", response.status()));
    }

    let bytes = response
        .bytes()
        .map_err(|e| format!("读取下载内容失败: {}", e))?;

    let tmp_path = dest.with_extension("tmp");
    std::fs::write(&tmp_path, &bytes).map_err(|e| format!("写入临时文件失败: {}", e))?;
    std::fs::rename(&tmp_path, dest).map_err(|e| format!("写入目标文件失败: {}", e))?;

    Ok(())
}

fn ensure_kokoro_local_assets(
    model_path_opt: Option<PathBuf>,
    voices_path_opt: Option<PathBuf>,
) -> Result<(PathBuf, PathBuf), String> {
    let cache_dir = default_kokoro_cache_dir();

    let model_path = model_path_opt.unwrap_or_else(|| cache_dir.join("kokoro-v1.0.onnx"));
    let voices_path = voices_path_opt.unwrap_or_else(|| cache_dir.join("voices-v1.0.bin"));

    if !model_path.exists() {
        download_to_file(KOKORO_MODEL_URL, &model_path).map_err(|e| {
            format!("下载 Kokoro 模型失败（{}）: {}", KOKORO_MODEL_URL, e)
        })?;
    }

    if !voices_path.exists() {
        download_to_file(KOKORO_VOICES_URL, &voices_path).map_err(|e| {
            format!("下载 Kokoro voices 失败（{}）: {}", KOKORO_VOICES_URL, e)
        })?;
    }

    Ok((model_path, voices_path))
}

fn split_text_for_tts(text: &str, min_chars: usize, max_chars: usize) -> Vec<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let mut segments: Vec<String> = Vec::new();
    let mut current = String::new();

    // Hard boundaries always split (colons introduce a new clause)
    let is_hard_boundary = |ch: char| matches!(ch, ':' | '：');

    let is_soft_boundary = |ch: char| {
        matches!(
            ch,
            '.' | '!' | '?' | ';' | '。' | '！' | '？' | '；' | '\n'
        )
    };

    for ch in trimmed.chars() {
        current.push(ch);
        let current_len = current.chars().count();

        let should_split = current_len >= max_chars
            || (current_len >= min_chars && is_soft_boundary(ch))
            || (is_hard_boundary(ch) && current_len >= 2);

        if should_split {
            let piece = current.trim();
            if !piece.is_empty() {
                segments.push(piece.to_string());
            }
            current.clear();
        }
    }

    let rest = current.trim();
    if !rest.is_empty() {
        if let Some(last) = segments.last_mut() {
            if rest.chars().count() < (min_chars / 2) {
                last.push(' ');
                last.push_str(rest);
            } else {
                segments.push(rest.to_string());
            }
        } else {
            segments.push(rest.to_string());
        }
    }

    segments
}

fn normalize_tts_language(language: &str) -> String {
    match language.to_ascii_lowercase().as_str() {
        "zh" | "zh-cn" => "zh-cn".to_string(),
        "en" | "en-us" => "en-us".to_string(),
        "ja" | "ja-jp" => "ja".to_string(),
        other => other.to_string(),
    }
}

fn strip_markdown_code_fence(content: &str) -> String {
    let trimmed = content.trim();
    if !trimmed.starts_with("```") {
        return trimmed.to_string();
    }

    let mut lines = trimmed.lines();
    let _ = lines.next();
    let mut body: Vec<&str> = Vec::new();
    for line in lines {
        if line.trim() == "```" {
            break;
        }
        body.push(line);
    }
    body.join("\n").trim().to_string()
}

fn extract_json_text_from_chat_response(value: &Value) -> Result<String, String> {
    let choices = value
        .get("choices")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "LLM 响应格式错误：缺少 choices".to_string())?;
    let first = choices
        .first()
        .ok_or_else(|| "LLM 未返回可用内容".to_string())?;
    let message = first
        .get("message")
        .ok_or_else(|| "LLM 响应格式错误：缺少 message".to_string())?;

    // Prefer tool call arguments when present (strongly structured output)
    if let Some(tool_calls) = message.get("tool_calls").and_then(|v| v.as_array()) {
        if let Some(first_call) = tool_calls.first() {
            if let Some(arguments) = first_call
                .get("function")
                .and_then(|f| f.get("arguments"))
                .and_then(|a| a.as_str())
            {
                return Ok(arguments.to_string());
            }
        }
    }

    let content = message
        .get("content")
        .ok_or_else(|| "LLM 响应格式错误：缺少 content".to_string())?;

    if let Some(text) = content.as_str() {
        return Ok(strip_markdown_code_fence(text));
    }

    if let Some(parts) = content.as_array() {
        let mut text = String::new();
        for part in parts {
            if let Some(part_text) = part.get("text").and_then(|t| t.as_str()) {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(part_text);
            }
        }
        if !text.trim().is_empty() {
            return Ok(strip_markdown_code_fence(&text));
        }
    }

    Err("LLM 返回内容为空".to_string())
}

fn estimate_tts_seconds_for_text(text: &str) -> f64 {
    let clean = text.trim();
    if clean.is_empty() {
        return 0.0;
    }
    let has_cjk = clean.chars().any(|c| {
        let cp = c as u32;
        (0x4E00..=0x9FFF).contains(&cp)
    });
    let cps = if has_cjk { 4.2 } else { 11.0 };
    let base = (clean.chars().count() as f64) / cps;
    base.max(0.4)
}

fn estimate_script_duration_seconds(lines: &[PodcastLine], pause_ms: u32, speed: f64) -> f64 {
    if lines.is_empty() {
        return 0.0;
    }
    let speed = if speed.is_finite() {
        speed.max(0.6).min(1.6)
    } else {
        1.0
    };
    let speech = lines
        .iter()
        .map(|l| estimate_tts_seconds_for_text(&l.text) / speed)
        .sum::<f64>();
    let pauses = ((lines.len().saturating_sub(1)) as f64) * ((pause_ms as f64) / 1000.0);
    speech + pauses
}

fn script_matches_language(lines: &[PodcastLine], language: &str) -> bool {
    if lines.is_empty() {
        return false;
    }
    let text = lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let total_chars = text.chars().count().max(1) as f64;
    let cjk_count = text
        .chars()
        .filter(|c| {
            let cp = *c as u32;
            (0x4E00..=0x9FFF).contains(&cp)
        })
        .count() as f64;
    let kana_count = text
        .chars()
        .filter(|c| {
            let cp = *c as u32;
            (0x3040..=0x309F).contains(&cp) || (0x30A0..=0x30FF).contains(&cp)
        })
        .count() as f64;
    let latin_count = text
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .count() as f64;

    let cjk_ratio = cjk_count / total_chars;
    let kana_ratio = kana_count / total_chars;
    let latin_ratio = latin_count / total_chars;

    if language.starts_with("en") {
        return latin_ratio >= 0.35 && cjk_ratio <= 0.12 && kana_ratio <= 0.05;
    }
    if language.starts_with("zh") {
        return cjk_ratio >= 0.22;
    }
    if language.starts_with("ja") {
        return (kana_ratio + cjk_ratio) >= 0.18;
    }
    true
}

fn extract_target_duration_seconds(topic: &str, style: &str) -> Option<u32> {
    let markers = ["分钟", "分鐘", "minutes", "minute", "mins", "min"];
    let find_in = |text: &str| -> Option<u32> {
        let lower = text.to_lowercase();
        for marker in markers {
            let mut search_start = 0usize;
            while let Some(pos) = lower[search_start..].find(marker) {
                let idx = search_start + pos;
                let bytes = lower.as_bytes();
                let mut end = idx;
                while end > 0 && bytes[end - 1].is_ascii_whitespace() {
                    end -= 1;
                }
                let mut start = end;
                while start > 0 && bytes[start - 1].is_ascii_digit() {
                    start -= 1;
                }
                if start < end {
                    if let Ok(v) = lower[start..end].parse::<u32>() {
                        if (1..=180).contains(&v) {
                            return Some(v * 60);
                        }
                    }
                }
                search_start = idx + marker.len();
            }
        }
        None
    };

    find_in(topic).or_else(|| find_in(style))
}

fn build_fallback_outline(
    topic: &str,
    style: &str,
    target_duration_seconds: Option<u32>,
) -> Vec<PodcastOutlineSection> {
    let total = target_duration_seconds.unwrap_or(420).clamp(180, 3600);
    let opening = ((total as f64) * 0.2).round() as u32;
    let core = ((total as f64) * 0.55).round() as u32;
    let closing = total.saturating_sub(opening + core).max(30);
    vec![
        PodcastOutlineSection {
            title: "Introduction and framing".to_string(),
            objective: format!("Set context for {topic}, define scope, and establish the discussion style: {style}."),
            target_seconds: opening,
        },
        PodcastOutlineSection {
            title: "Core discussion and analysis".to_string(),
            objective: format!("Develop the main arguments around {topic} with examples, trade-offs, and practical implications."),
            target_seconds: core,
        },
        PodcastOutlineSection {
            title: "Summary and takeaways".to_string(),
            objective: format!("Conclude {topic} with clear key points, actionable advice, and a concise closing."),
            target_seconds: closing,
        },
    ]
}

fn generate_podcast_outline_with_llm(
    client: &reqwest::blocking::Client,
    endpoint: &str,
    api_key: &str,
    model: &str,
    topic: &str,
    style: &str,
    language: &str,
    roles: &[PodcastRole],
    target_duration_seconds: Option<u32>,
) -> Result<Vec<PodcastOutlineSection>, String> {
    let Some(target) = target_duration_seconds else {
        return Ok(build_fallback_outline(topic, style, target_duration_seconds));
    };

    let role_desc = roles
        .iter()
        .map(|r| {
            let persona = r.persona.as_deref().unwrap_or("N/A");
            format!("- role: {}, persona: {}", r.name, persona)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let user_prompt = format!(
        "Create a podcast outline.\nLanguage: {language}\nTopic: {topic}\nStyle: {style}\nRoles:\n{role_desc}\nTarget duration: {target} seconds.\n\nRules:\n1) Return 3 to 6 sections.\n2) Each section must have title, objective, and target_seconds.\n3) Sum of target_seconds should be close to target duration.\n4) Keep objective specific and on-topic.\n5) Use tool call submit_podcast_outline."
    );

    let body = serde_json::json!({
        "model": model,
        "temperature": 0.4,
        "messages": [
            { "role": "system", "content": "You are a podcast planner. Use tool calls only." },
            { "role": "user", "content": user_prompt }
        ],
        "tools": [
            {
                "type": "function",
                "function": {
                    "name": "submit_podcast_outline",
                    "description": "Submit podcast outline sections with target durations",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "sections": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "title": { "type": "string" },
                                        "objective": { "type": "string" },
                                        "target_seconds": { "type": "integer" }
                                    },
                                    "required": ["title", "objective", "target_seconds"],
                                    "additionalProperties": false
                                }
                            }
                        },
                        "required": ["sections"],
                        "additionalProperties": false
                    }
                }
            }
        ],
        "tool_choice": {
            "type": "function",
            "function": { "name": "submit_podcast_outline" }
        }
    });

    let response = client
        .post(endpoint)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .map_err(|e| format!("请求 LLM 大纲失败: {}", e))?;
    if !response.status().is_success() {
        let code = response.status();
        let text = response.text().unwrap_or_default();
        return Err(format!("LLM 大纲返回错误 {}: {}", code, text));
    }

    let value: Value = response
        .json()
        .map_err(|e| format!("解析 LLM 大纲响应失败: {}", e))?;
    let json_text = extract_json_text_from_chat_response(&value)?;
    let parsed: Value = serde_json::from_str(&json_text)
        .map_err(|e| format!("解析大纲 JSON 失败: {}", e))?;
    let sections = parsed
        .get("sections")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "大纲缺少 sections 数组".to_string())?;

    let mut out = Vec::new();
    for sec in sections {
        let title = sec
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .trim()
            .to_string();
        let objective = sec
            .get("objective")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .trim()
            .to_string();
        let target_seconds = sec
            .get("target_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        if title.is_empty() || objective.is_empty() || target_seconds == 0 {
            continue;
        }
        out.push(PodcastOutlineSection {
            title,
            objective,
            target_seconds: target_seconds.clamp(20, 3600),
        });
    }
    if out.is_empty() {
        return Err("LLM 未返回有效大纲".to_string());
    }
    if out.len() > 6 {
        out.truncate(6);
    }

    let sum: u32 = out.iter().map(|s| s.target_seconds).sum();
    if sum == 0 {
        return Ok(build_fallback_outline(topic, style, target_duration_seconds));
    }
    let scale = (target as f64) / (sum as f64);
    for sec in &mut out {
        sec.target_seconds = ((sec.target_seconds as f64) * scale).round() as u32;
        sec.target_seconds = sec.target_seconds.clamp(20, target.max(20));
    }
    let adjusted_sum: u32 = out.iter().map(|s| s.target_seconds).sum();
    if adjusted_sum < target {
        if let Some(last) = out.last_mut() {
            last.target_seconds += target - adjusted_sum;
        }
    } else if adjusted_sum > target {
        let overflow = adjusted_sum - target;
        if let Some(last) = out.last_mut() {
            last.target_seconds = last.target_seconds.saturating_sub(overflow).max(20);
        }
    }
    Ok(out)
}

fn normalize_line_key(line: &PodcastLine) -> String {
    line.text
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_podcast_lines(lines: &[Value], roles: &[PodcastRole]) -> Vec<PodcastLine> {
    let mut out: Vec<PodcastLine> = Vec::new();
    for item in lines {
        let role_name = item
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .trim()
            .to_string();
        let text = item
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .trim()
            .to_string();
        if role_name.is_empty() || text.is_empty() {
            continue;
        }
        if let Some(role) = roles.iter().find(|r| r.name == role_name) {
            out.push(PodcastLine {
                role: role_name,
                voice: role.voice.clone(),
                text,
            });
        }
    }
    out
}

fn progress_phase_hint(progress_ratio: f64) -> &'static str {
    if progress_ratio < 0.30 {
        "Opening phase: establish context, key definitions, and discussion roadmap."
    } else if progress_ratio < 0.80 {
        "Middle phase: deepen analysis with evidence, examples, and balanced viewpoints."
    } else {
        "Closing phase: synthesize conclusions, practical actions, and clear takeaways."
    }
}

fn extend_podcast_script_with_llm(
    client: &reqwest::blocking::Client,
    endpoint: &str,
    api_key: &str,
    model: &str,
    topic: &str,
    style: &str,
    language: &str,
    speed: f64,
    pause_ms: u32,
    target_duration_seconds: u32,
    section_title: Option<&str>,
    section_objective: Option<&str>,
    section_target_seconds: Option<u32>,
    roles: &[PodcastRole],
    existing_lines: &[PodcastLine],
) -> Result<Vec<PodcastLine>, String> {
    let elapsed = estimate_script_duration_seconds(existing_lines, pause_ms, speed);
    let target = target_duration_seconds as f64;
    let remaining = (target - elapsed).max(0.0);
    let progress_ratio = if target > 0.0 { (elapsed / target).clamp(0.0, 1.0) } else { 0.0 };
    let role_desc = roles
        .iter()
        .map(|r| {
            let persona = r.persona.as_deref().unwrap_or("N/A");
            format!("- role: {}, persona: {}", r.name, persona)
        })
        .collect::<Vec<_>>()
        .join("\n");
    let transcript_tail = existing_lines
        .iter()
        .rev()
        .take(28)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|l| format!("{}: {}", l.role, l.text))
        .collect::<Vec<_>>()
        .join("\n");
    let phase_hint = progress_phase_hint(progress_ratio);
    let section_context = match (section_title, section_objective, section_target_seconds) {
        (Some(title), Some(objective), Some(sec_target)) => format!(
            "Current section:\n- title: {title}\n- objective: {objective}\n- section_target_seconds: {sec_target}\n"
        ),
        _ => String::new(),
    };
    let desired_new_lines = ((remaining / ((6.8_f64 / speed.max(0.6).min(1.6)) + (pause_ms as f64 / 1000.0)))
        .round() as i32)
        .clamp(8, 64);

    let system_prompt =
        "You are a podcast continuation planner. Use tool calls only. Never answer with plain text.";
    let user_prompt = format!(
        "Continue an existing podcast dialogue.\nLanguage: {language}\nTopic: {topic}\nStyle: {style}\nRoles:\n{role_desc}\n\nCurrent progress:\n- elapsed_seconds: {elapsed:.1}\n- target_seconds: {target:.1}\n- remaining_seconds: {remaining:.1}\n- progress_ratio: {progress_ratio:.2}\n- phase: {phase_hint}\n{section_context}\nRecent transcript (must continue naturally, no reset):\n{transcript_tail}\n\nRules:\n1) Stay strictly on topic and maintain narrative continuity.\n2) Do not repeat previous lines or restart introduction.\n3) Produce about {desired_new_lines} NEW lines.\n4) Role must be from provided roles only.\n5) Keep each line concise (<= 120 chars).\n6) Predict and prepare the next part naturally while completing this section objective.\n7) Use tool calls. You may call estimate_script_duration, then call submit_podcast_script with new lines only."
    );
    let mut messages = vec![
        serde_json::json!({ "role": "system", "content": system_prompt }),
        serde_json::json!({ "role": "user", "content": user_prompt }),
    ];

    let mut out: Option<Vec<PodcastLine>> = None;
    for _round in 0..6 {
        let body = serde_json::json!({
            "model": model,
            "temperature": 0.5,
            "messages": messages,
            "tools": [
                {
                    "type": "function",
                    "function": {
                        "name": "estimate_script_duration",
                        "description": "Estimate spoken duration in seconds for candidate lines",
                        "parameters": {
                            "type": "object",
                            "properties": {
                                "lines": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "text": { "type": "string" }
                                        },
                                        "required": ["text"],
                                        "additionalProperties": false
                                    }
                                }
                            },
                            "required": ["lines"],
                            "additionalProperties": false
                        }
                    }
                },
                {
                    "type": "function",
                    "function": {
                        "name": "submit_podcast_script",
                        "description": "Submit only newly generated continuation lines",
                        "parameters": {
                            "type": "object",
                            "properties": {
                                "lines": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "role": { "type": "string" },
                                            "text": { "type": "string" }
                                        },
                                        "required": ["role", "text"],
                                        "additionalProperties": false
                                    }
                                }
                            },
                            "required": ["lines"],
                            "additionalProperties": false
                        }
                    }
                }
            ],
            "tool_choice": "auto"
        });

        let resp = client
            .post(endpoint)
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .map_err(|e| format!("请求 LLM 失败: {}", e))?;
        if !resp.status().is_success() {
            let code = resp.status();
            let text = resp.text().unwrap_or_default();
            return Err(format!("LLM 返回错误 {}: {}", code, text));
        }
        let value: Value = resp
            .json()
            .map_err(|e| format!("解析 LLM 响应失败: {}", e))?;

        let choices = value
            .get("choices")
            .and_then(|v| v.as_array())
            .ok_or_else(|| "LLM 响应格式错误：缺少 choices".to_string())?;
        let first = choices
            .first()
            .ok_or_else(|| "LLM 未返回可用内容".to_string())?;
        let message = first
            .get("message")
            .cloned()
            .ok_or_else(|| "LLM 响应格式错误：缺少 message".to_string())?;

        if let Some(tool_calls) = message.get("tool_calls").and_then(|v| v.as_array()) {
            messages.push(serde_json::json!({
                "role": "assistant",
                "content": message.get("content").cloned().unwrap_or(Value::Null),
                "tool_calls": tool_calls
            }));
            for tool_call in tool_calls {
                let tool_call_id = tool_call
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let function_name = tool_call
                    .get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let arguments_text = tool_call
                    .get("function")
                    .and_then(|f| f.get("arguments"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("{}");
                let arguments: Value =
                    serde_json::from_str(arguments_text).unwrap_or_else(|_| serde_json::json!({}));

                match function_name.as_str() {
                    "estimate_script_duration" => {
                        let candidate_lines = arguments
                            .get("lines")
                            .and_then(|v| v.as_array())
                            .cloned()
                            .unwrap_or_default();
                        let pseudo_lines: Vec<PodcastLine> = candidate_lines
                            .iter()
                            .filter_map(|item| {
                                let text = item
                                    .get("text")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or_default()
                                    .trim()
                                    .to_string();
                                if text.is_empty() {
                                    return None;
                                }
                                Some(PodcastLine {
                                    role: String::new(),
                                    voice: String::new(),
                                    text,
                                })
                            })
                            .collect();
                        let estimated_seconds =
                            estimate_script_duration_seconds(&pseudo_lines, pause_ms, speed);
                        messages.push(serde_json::json!({
                            "role": "tool",
                            "tool_call_id": tool_call_id,
                            "name": "estimate_script_duration",
                            "content": serde_json::json!({
                                "estimated_seconds": estimated_seconds,
                                "estimated_minutes": estimated_seconds / 60.0
                            }).to_string()
                        }));
                    }
                    "submit_podcast_script" => {
                        let lines = arguments
                            .get("lines")
                            .and_then(|v| v.as_array())
                            .cloned()
                            .unwrap_or_default();
                        let parsed = parse_podcast_lines(&lines, roles);
                        if !parsed.is_empty() {
                            out = Some(parsed);
                            break;
                        }
                        messages.push(serde_json::json!({
                            "role": "tool",
                            "tool_call_id": tool_call_id,
                            "name": "submit_podcast_script",
                            "content": "{\"ok\":false,\"error\":\"empty_or_invalid_lines\"}"
                        }));
                    }
                    _ => {
                        messages.push(serde_json::json!({
                            "role": "tool",
                            "tool_call_id": tool_call_id,
                            "name": function_name,
                            "content": "{\"ok\":false,\"error\":\"unknown_tool\"}"
                        }));
                    }
                }
            }
            if out.is_some() {
                break;
            }
        }
    }

    let mut new_lines = out.unwrap_or_default();
    if !script_matches_language(&new_lines, language) {
        return Err(format!("续写语言不符合要求（目标语言: {}）", language));
    }
    let existing_keys: HashSet<String> = existing_lines.iter().map(normalize_line_key).collect();
    new_lines.retain(|line| !existing_keys.contains(&normalize_line_key(line)));
    Ok(new_lines)
}

fn generate_podcast_script_with_llm(
    api_base: &str,
    api_key: &str,
    model: &str,
    topic: &str,
    style: &str,
    language: &str,
    speed: f64,
    pause_ms: u32,
    target_duration_seconds: Option<u32>,
    roles: &[PodcastRole],
    progress_cb: &mut dyn FnMut(f64, String, String),
) -> Result<(Vec<PodcastLine>, Vec<PodcastOutlineSection>), String> {
    let role_desc = roles
        .iter()
        .map(|r| {
            let persona = r.persona.as_deref().unwrap_or("N/A");
            format!("- role: {}, persona: {}", r.name, persona)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let endpoint = format!("{}/chat/completions", api_base.trim_end_matches('/'));

    let client = reqwest::blocking::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .map_err(|e| format!("初始化 LLM 客户端失败: {}", e))?;

    progress_cb(8.0, "outline".to_string(), "Planning episode outline...".to_string());
    let outline = match generate_podcast_outline_with_llm(
        &client,
        &endpoint,
        api_key,
        model,
        topic,
        style,
        language,
        roles,
        target_duration_seconds,
    ) {
        Ok(v) => v,
        Err(e) => {
            log_stderr(&format!("[PodcastAgent] LLM outline failed, fallback outline: {}", e));
            build_fallback_outline(topic, style, target_duration_seconds)
        }
    };
    progress_cb(
        16.0,
        "outline".to_string(),
        format!("Outline ready ({} sections)", outline.len()),
    );

    let duration_hint = target_duration_seconds.map(|secs| {
        let mins = (secs as f64) / 60.0;
        let per_line = (6.8_f64 / speed.max(0.6).min(1.6)) + ((pause_ms as f64) / 1000.0);
        let target_lines = ((secs as f64) / per_line).round().clamp(12.0, 220.0) as usize;
        (mins, target_lines)
    });
    // Product constraint: never regenerate the whole script after initial generation.
    let max_script_attempt = 1usize;

    let system_prompt = "You are a podcast script planner. Use tool calls only. Never answer with plain text.";
    let mut last_err = String::new();
    for script_attempt in 0..max_script_attempt {
        let strict = script_attempt > 0;
        let duration_rule = if let Some((mins, target_lines)) = duration_hint {
            if strict {
                format!(
                    "Duration target: about {:.1} minutes. Estimated duration must be within 90% to 115% of target. You MUST provide enough content and at least {} lines to match the target duration.",
                    mins, target_lines,
                )
            } else {
                format!(
                    "Duration target: about {:.1} minutes. Choose a suitable number of lines to approximately match this duration and validate with tool calls.",
                    mins
                )
            }
        } else {
            "Decide dialogue length naturally based on topic complexity and style.".to_string()
        };

        let user_prompt = format!(
            "Create a podcast dialogue.\nLanguage: {language}\nTopic: {topic}\nStyle: {style}\nRoles:\n{role_desc}\n\nRules:\n1) role must be from provided roles only.\n2) {duration_rule}\n3) Keep each line concise (<= 120 chars).\n4) Alternate roles naturally.\n5) You MUST use tool calls: call estimate_script_duration first when unsure, then call submit_podcast_script."
        );
        let mut messages = vec![
            serde_json::json!({ "role": "system", "content": system_prompt }),
            serde_json::json!({ "role": "user", "content": user_prompt }),
        ];

        let mut out: Option<Vec<PodcastLine>> = None;
        for _round in 0..8 {
            let body = serde_json::json!({
                "model": model,
                "temperature": if strict { 0.45 } else { 0.7 },
                "messages": messages,
                "tools": [
                    {
                        "type": "function",
                        "function": {
                            "name": "estimate_script_duration",
                            "description": "Estimate spoken duration in seconds for candidate lines",
                            "parameters": {
                                "type": "object",
                                "properties": {
                                    "lines": {
                                        "type": "array",
                                        "items": {
                                            "type": "object",
                                            "properties": {
                                                "text": { "type": "string" }
                                            },
                                            "required": ["text"],
                                            "additionalProperties": false
                                        }
                                    }
                                },
                                "required": ["lines"],
                                "additionalProperties": false
                            }
                        }
                    },
                    {
                        "type": "function",
                        "function": {
                            "name": "submit_podcast_script",
                            "description": "Submit the final structured podcast script",
                            "parameters": {
                                "type": "object",
                                "properties": {
                                    "lines": {
                                        "type": "array",
                                        "items": {
                                            "type": "object",
                                            "properties": {
                                                "role": { "type": "string" },
                                                "text": { "type": "string" }
                                            },
                                            "required": ["role", "text"],
                                            "additionalProperties": false
                                        }
                                    }
                                },
                                "required": ["lines"],
                                "additionalProperties": false
                            }
                        }
                    }
                ],
                "tool_choice": "auto"
            });

            let mut value: Option<Value> = None;
            for attempt in 1..=3 {
                let response = client
                    .post(&endpoint)
                    .bearer_auth(api_key)
                    .json(&body)
                    .send();
                match response {
                    Ok(resp) if resp.status().is_success() => {
                        let v: Value = resp
                            .json()
                            .map_err(|e| format!("解析 LLM 响应失败: {}", e))?;
                        value = Some(v);
                        break;
                    }
                    Ok(resp) => {
                        let code = resp.status();
                        let text = resp.text().unwrap_or_default();
                        last_err = format!("LLM 返回错误 {}: {}", code, text);
                    }
                    Err(e) => {
                        last_err = format!("请求 LLM 失败: {}", e);
                    }
                }
                if attempt < 3 {
                    std::thread::sleep(std::time::Duration::from_millis(600 * attempt as u64));
                }
            }

            let value = value.ok_or_else(|| last_err.clone())?;
            let choices = value
                .get("choices")
                .and_then(|v| v.as_array())
                .ok_or_else(|| "LLM 响应格式错误：缺少 choices".to_string())?;
            let first = choices
                .first()
                .ok_or_else(|| "LLM 未返回可用内容".to_string())?;
            let message = first
                .get("message")
                .cloned()
                .ok_or_else(|| "LLM 响应格式错误：缺少 message".to_string())?;

            let mut used_tool = false;
            if let Some(tool_calls) = message.get("tool_calls").and_then(|v| v.as_array()) {
                messages.push(serde_json::json!({
                    "role": "assistant",
                    "content": message.get("content").cloned().unwrap_or(Value::Null),
                    "tool_calls": tool_calls
                }));

                for tool_call in tool_calls {
                    let tool_call_id = tool_call
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let function_name = tool_call
                        .get("function")
                        .and_then(|f| f.get("name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let arguments_text = tool_call
                        .get("function")
                        .and_then(|f| f.get("arguments"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("{}");
                    let arguments: Value =
                        serde_json::from_str(arguments_text).unwrap_or_else(|_| serde_json::json!({}));

                    match function_name.as_str() {
                        "estimate_script_duration" => {
                            let candidate_lines = arguments
                                .get("lines")
                                .and_then(|v| v.as_array())
                                .cloned()
                                .unwrap_or_default();
                            let pseudo_lines: Vec<PodcastLine> = candidate_lines
                                .iter()
                                .filter_map(|item| {
                                    let text = item
                                        .get("text")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or_default()
                                        .trim()
                                        .to_string();
                                    if text.is_empty() {
                                        return None;
                                    }
                                    Some(PodcastLine {
                                        role: String::new(),
                                        voice: String::new(),
                                        text,
                                    })
                                })
                                .collect();
                            let estimated_seconds =
                                estimate_script_duration_seconds(&pseudo_lines, pause_ms, speed);
                            messages.push(serde_json::json!({
                                "role": "tool",
                                "tool_call_id": tool_call_id,
                                "name": "estimate_script_duration",
                                "content": serde_json::json!({
                                    "estimated_seconds": estimated_seconds,
                                    "estimated_minutes": estimated_seconds / 60.0,
                                    "target_seconds": target_duration_seconds,
                                    "speed": speed,
                                    "pause_ms": pause_ms
                                }).to_string()
                            }));
                            used_tool = true;
                        }
                        "submit_podcast_script" => {
                            let lines = arguments
                                .get("lines")
                                .and_then(|v| v.as_array())
                                .ok_or_else(|| "脚本缺少 lines 数组".to_string())?;
                            let parsed_lines = parse_podcast_lines(lines, roles);
                            if !parsed_lines.is_empty() {
                                out = Some(parsed_lines);
                                break;
                            }
                            messages.push(serde_json::json!({
                                "role": "tool",
                                "tool_call_id": tool_call_id,
                                "name": "submit_podcast_script",
                                "content": "{\"ok\":false,\"error\":\"empty_or_invalid_lines\"}"
                            }));
                            used_tool = true;
                        }
                        _ => {
                            messages.push(serde_json::json!({
                                "role": "tool",
                                "tool_call_id": tool_call_id,
                                "name": function_name,
                                "content": "{\"ok\":false,\"error\":\"unknown_tool\"}"
                            }));
                            used_tool = true;
                        }
                    }
                }
            } else {
                let content = extract_json_text_from_chat_response(&value)?;
                let parsed: Value = serde_json::from_str(&content)
                    .map_err(|e| format!("解析脚本 JSON 失败: {}", e))?;
                if let Some(lines) = parsed.get("lines").and_then(|v| v.as_array()) {
                    let parsed_lines = parse_podcast_lines(lines, roles);
                    if !parsed_lines.is_empty() {
                        out = Some(parsed_lines);
                        break;
                    }
                }
            }

            if !used_tool {
                break;
            }
        }
        let mut out = out.unwrap_or_default();

        if out.is_empty() {
            last_err = "LLM 未生成有效对话行".to_string();
            continue;
        }

        let max_lines = if target_duration_seconds.is_some() { 220 } else { 120 };
        if out.len() > max_lines {
            out.truncate(max_lines);
        }

        if let Some(target) = target_duration_seconds {
            let estimated = estimate_script_duration_seconds(&out, pause_ms, speed);
            if estimated < (target as f64) * 0.90 {
                last_err = format!(
                    "LLM 输出过短（预估 {:.1}s，目标 {}s）",
                    estimated, target
                );
                continue;
            }
            if estimated > (target as f64) * 1.20 {
                last_err = format!(
                    "LLM 输出过长（预估 {:.1}s，目标 {}s）",
                    estimated, target
                );
                continue;
            }
        }

        if !script_matches_language(&out, language) {
            last_err = format!("LLM 输出语言不符合要求（目标语言: {}）", language);
            continue;
        }

        if let Some(target) = target_duration_seconds {
            let mut estimated = estimate_script_duration_seconds(&out, pause_ms, speed);
            let mut cumulative_target = 0u32;
            let section_total = outline.len().max(1);
            for (section_idx, section) in outline.iter().enumerate() {
                let section_progress = 16.0 + ((section_idx as f64) / (section_total as f64)) * 44.0;
                progress_cb(
                    section_progress,
                    format!("section:{}:{}", section_idx + 1, section_total),
                    format!("Writing section {}: {}", section_idx + 1, section.title),
                );
                cumulative_target = (cumulative_target + section.target_seconds).min(target);
                let section_goal = (cumulative_target as f64) * 0.93;
                let mut section_round = 0usize;
                while estimated < section_goal && section_round < 3 && out.len() < 260 {
                    let extra_lines = match extend_podcast_script_with_llm(
                        &client,
                        &endpoint,
                        api_key,
                        model,
                        topic,
                        style,
                        language,
                        speed,
                        pause_ms,
                        target,
                        Some(&section.title),
                        Some(&section.objective),
                        Some(section.target_seconds),
                        roles,
                        &out,
                    ) {
                        Ok(v) => v,
                        Err(e) => {
                            log_stderr(&format!("[PodcastAgent] LLM continuation failed: {}", e));
                            break;
                        }
                    };
                    if extra_lines.is_empty() {
                        log_stderr("[PodcastAgent] LLM continuation returned empty lines");
                        break;
                    }

                    let mut seen: HashSet<String> = out.iter().map(normalize_line_key).collect();
                    let mut appended = 0usize;
                    for line in extra_lines {
                        let key = normalize_line_key(&line);
                        if !seen.contains(&key) {
                            seen.insert(key);
                            out.push(line);
                            appended += 1;
                        }
                    }
                    if appended == 0 {
                        log_stderr("[PodcastAgent] LLM continuation produced duplicated lines");
                        break;
                    }
                    if out.len() > 260 {
                        out.truncate(260);
                    }
                    estimated = estimate_script_duration_seconds(&out, pause_ms, speed);
                    section_round += 1;
                }
            }

            if estimated < (target as f64) * 0.90 {
                last_err = format!(
                    "LLM 续写后仍过短（预估 {:.1}s，目标 {}s）",
                    estimated, target
                );
                continue;
            }
        }
        progress_cb(62.0, "script".to_string(), "Script ready. Starting synthesis...".to_string());
        return Ok((out, outline.clone()));
    }

    Err(last_err)
}

fn build_fallback_podcast_script(
    topic: &str,
    style: &str,
    language: &str,
    target_duration_seconds: Option<u32>,
    pause_ms: u32,
    speed: f64,
    roles: &[PodcastRole],
) -> Vec<PodcastLine> {
    if roles.is_empty() {
        return Vec::new();
    }
    let mut lines: Vec<PodcastLine> = Vec::new();
    let opening = if language.starts_with("en") {
        format!("Today we discuss {}. The style is {}.", topic, style)
    } else if language.starts_with("ja") {
        format!("今日は「{}」について話します。スタイルは{}です。", topic, style)
    } else {
        format!("今天聊聊{}。风格是{}。", topic, style)
    };
    let host = &roles[0];
    lines.push(PodcastLine {
        role: host.name.clone(),
        voice: host.voice.clone(),
        text: opening,
    });

    let templates_zh = [
        "先给一个核心观点：{} 的关键在于目标明确与节奏管理。",
        "从实操看，第一步建议先拆解问题，再验证最小可行方案。",
        "很多人会忽略风险边界，我们可以补充下常见误区。",
        "如果资源有限，优先做收益最高的 20% 动作。",
        "再往下可以用一个例子，让听众更容易落地。",
        "最后我们给出 3 条可执行建议，方便直接开始。",
    ];
    let templates_en = [
        "A core point first: success in {} depends on clear goals and execution rhythm.",
        "From practice, step one is to break the problem down and validate a minimum viable path.",
        "People often miss risk boundaries, so let us surface the common pitfalls.",
        "If resources are limited, prioritize the top 20 percent actions with the highest impact.",
        "Next, we can add a concrete example so listeners can apply it immediately.",
        "Finally, we will summarize three actionable takeaways.",
    ];
    let templates_ja = [
        "まず核心です。{} の鍵は、目的の明確化と実行のリズムです。",
        "実務では、最初に課題を分解し、最小実行可能な案を検証するのが有効です。",
        "見落とされがちなリスク境界も、ここで整理しておきましょう。",
        "リソースが限られる場合は、効果の高い 20% を優先します。",
        "次に、すぐ使える具体例を一つ入れて理解を深めます。",
        "最後に、実行しやすい 3 つの要点にまとめます。",
    ];
    let templates = if language.starts_with("en") {
        &templates_en
    } else if language.starts_with("ja") {
        &templates_ja
    } else {
        &templates_zh
    };

    let per_line = (6.8_f64 / speed.max(0.6).min(1.6)) + ((pause_ms as f64) / 1000.0);
    let turns = target_duration_seconds
        .map(|secs| ((secs as f64 / per_line).round() as usize).clamp(12, 220))
        .unwrap_or(12usize);
    for idx in 1..turns {
        let role = &roles[idx % roles.len()];
        let tpl = templates[idx % templates.len()];
        lines.push(PodcastLine {
            role: role.name.clone(),
            voice: role.voice.clone(),
            text: tpl.replacen("{}", topic, 1),
        });
    }

    if let Some(target) = target_duration_seconds {
        let mut idx = turns;
        while estimate_script_duration_seconds(&lines, pause_ms, speed) < (target as f64) * 0.92
            && lines.len() < 260
        {
            let role = &roles[idx % roles.len()];
            let extra_text = if language.starts_with("en") {
                format!(
                    "Let us go deeper on {} with a practical case, trade-offs, and a concrete next step.",
                    topic
                )
            } else if language.starts_with("ja") {
                format!(
                    "{} について、具体例とトレードオフ、次の一手まで掘り下げて整理します。",
                    topic
                )
            } else {
                format!(
                    "我们继续围绕{}展开，补充案例、权衡点和下一步可执行方案。",
                    topic
                )
            };
            lines.push(PodcastLine {
                role: role.name.clone(),
                voice: role.voice.clone(),
                text: extra_text,
            });
            idx += 1;
        }
    }

    lines
}

fn write_stereo_wav_from_mono(path: &PathBuf, mono_samples: &[f32], sample_rate: u32) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(path, spec)
        .map_err(|e| format!("创建 WAV 文件失败: {}", e))?;

    for &sample in mono_samples {
        let s = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
        writer
            .write_sample(s)
            .and_then(|_| writer.write_sample(s))
            .map_err(|e| format!("写入 WAV 数据失败: {}", e))?;
    }

    writer
        .finalize()
        .map_err(|e| format!("完成 WAV 写入失败: {}", e))?;

    Ok(())
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
    let _ = use_gpu;

    match model.as_str() {
        "htdemucs_onnx" | "htdemucs" => {
            extract_bgm_onnx(
                app,
                window,
                input,
                output,
                "htdemucs".to_string(),
                "htdemucs_onnx".to_string(),
                format,
                bitrate,
                sample_rate,
                separation_mode,
            )
            .await
        }
        "htdemucs_ft_onnx" | "htdemucs_ft" => {
            extract_bgm_onnx(
                app,
                window,
                input,
                output,
                "htdemucs_ft".to_string(),
                "htdemucs_ft_onnx".to_string(),
                format,
                bitrate,
                sample_rate,
                separation_mode,
            )
            .await
        }
        "htdemucs_6s_onnx" | "htdemucs_6s" => {
            extract_bgm_onnx(
                app,
                window,
                input,
                output,
                "htdemucs_6s".to_string(),
                "htdemucs_6s_onnx".to_string(),
                format,
                bitrate,
                sample_rate,
                separation_mode,
            )
            .await
        }
        _ => Err(
            "当前版本仅支持 ONNX 模型：htdemucs_onnx / htdemucs_ft_onnx / htdemucs_6s_onnx（2/4/6 轨可选）"
                .to_string(),
        ),
    }
}

async fn extract_bgm_onnx(
    app: tauri::AppHandle,
    window: Window,
    input: String,
    output: Option<String>,
    model_key: String,
    output_tag: String,
    format: Option<String>,
    bitrate: Option<u32>,
    sample_rate: Option<u32>,
    separation_mode: Option<String>,
) -> Result<ExtractResult, String> {
    let input_path = PathBuf::from(&input);
    if !input_path.exists() {
        return Err(format!("文件不存在: {}", input));
    }

    ensure_ort_initialized(&app)?;

    let _ = window.emit(
        "extraction-progress",
        ProgressPayload {
            progress: 0.0,
            status: "正在初始化 ONNX 引擎...".to_string(),
            stage: "init".to_string(),
        },
    );

    let output_dir = if let Some(ref out) = output {
        PathBuf::from(out)
    } else {
        dirs::document_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
            .join("BGM Extractor Output")
    };

    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("无法获取文件名")?;

    let result_dir = output_dir.join(&output_tag).join(stem);
    std::fs::create_dir_all(&result_dir).map_err(|e| e.to_string())?;

    let mode = separation_mode.as_deref().unwrap_or("2-track");
    let supports_6_track = model_key == "htdemucs_6s";
    if mode == "6-track" && !supports_6_track {
        return Err("该 ONNX 模型不支持 6-track（仅支持 2-track/4-track）".to_string());
    }

    let model_path = get_onnx_model_path(&app, &model_key).ok_or_else(|| {
        format!(
            "找不到 ONNX 模型文件（onnx_models/{}）",
            get_onnx_model_filename(&model_key).unwrap_or("<unknown>")
        )
    })?;

    #[derive(Debug, Deserialize)]
    struct OnnxModelConfigFile {
        sample_rate: u32,
        channels: usize,
        sources: Vec<String>,
        nfft: usize,
        hop_length: usize,
        #[serde(default)]
        ensemble: Option<OnnxModelEnsemble>,
    }

    #[derive(Debug, Deserialize)]
    struct OnnxModelEnsemble {
        models: Vec<String>,
        weights: Vec<Vec<f32>>,
    }

    fn fallback_config(model_key: &str) -> DemucsConfig {
        match model_key {
            "htdemucs_6s" => DemucsConfig {
                sources: vec![
                    "drums".to_string(),
                    "bass".to_string(),
                    "other".to_string(),
                    "vocals".to_string(),
                    "guitar".to_string(),
                    "piano".to_string(),
                ],
                ..DemucsConfig::default()
            },
            _ => DemucsConfig::default(),
        }
    }

    let mut ensemble_models: Option<Vec<String>> = None;
    let mut ensemble_weights: Option<Vec<Vec<f32>>> = None;

    let mut config = if let Some(config_path) = get_onnx_config_path(&app, &model_key) {
        let bytes = std::fs::read(&config_path).map_err(|e| format!("读取配置失败: {}", e))?;
        let parsed: OnnxModelConfigFile = serde_json::from_slice(&bytes)
            .map_err(|e| format!("解析配置失败: {}", e))?;

        if let Some(ens) = parsed.ensemble {
            ensemble_models = Some(ens.models);
            ensemble_weights = Some(ens.weights);
        }

        let segment_samples = ((parsed.sample_rate as f32) * 7.8).round() as usize;

        DemucsConfig {
            sample_rate: parsed.sample_rate,
            channels: parsed.channels,
            sources: parsed.sources,
            nfft: parsed.nfft,
            hop_length: parsed.hop_length,
            segment_samples,
            shifts: 1,
            execution_provider: ExecutionProvider::Cpu,
        }
    } else {
        fallback_config(&model_key)
    };
    config.execution_provider = ExecutionProvider::Cpu;
    config.shifts = 1;

    let progress_cb: ProgressCallback = Box::new({
        let window = window.clone();
        move |progress, stage, status| {
            let _ = window.emit(
                "extraction-progress",
                ProgressPayload {
                    progress: progress.min(99.0) as f64,
                    status: status.to_string(),
                    stage: stage.to_string(),
                },
            );
        }
    });

    let _ = window.emit(
        "extraction-progress",
        ProgressPayload {
            progress: 5.0,
            status: "正在读取音频...".to_string(),
            stage: "loading".to_string(),
        },
    );

    let audio = onnx_demucs::read_audio(&input_path, config.sample_rate)
        .map_err(|e| format!("读取音频失败: {}", e))?;

    let sr = config.sample_rate;
    let model_dir = model_path
        .parent()
        .ok_or_else(|| "无法获取 ONNX 模型目录".to_string())?;

    let model_paths: Vec<PathBuf> = if let Some(files) = ensemble_models.as_ref() {
        let mut paths = Vec::with_capacity(files.len());
        for filename in files {
            let p = model_dir.join(filename);
            if !p.exists() {
                return Err(format!("找不到 ONNX ensemble 子模型文件: {}", p.display()));
            }
            paths.push(p);
        }
        paths
    } else {
        vec![model_path.clone()]
    };

    let mut demucs = OnnxDemucs::new(model_paths, config, ensemble_weights)
        .map_err(|e| format!("加载 ONNX 模型失败: {}", e))?;

    let sources = demucs
        .separate(&audio, Some(&progress_cb), None)
        .map_err(|e| format!("ONNX 分离失败: {}", e))?;

    let source_names = demucs.source_names().to_vec();
    let mut tracks: Vec<TrackInfo> = Vec::new();

    let find_idx = |name: &str| source_names.iter().position(|n| n == name);

    let get_source = |name: &str| -> Result<&ndarray::Array2<f32>, String> {
        let idx = find_idx(name).ok_or_else(|| format!("ONNX 模型输出缺少 {}", name))?;
        Ok(&sources[idx])
    };

    let sum_sources = |names: &[&str]| -> Result<ndarray::Array2<f32>, String> {
            let first = get_source(names[0])?;
            let mut out = ndarray::Array2::<f32>::zeros(first.raw_dim());
            for name in names {
                if let Some(idx) = find_idx(name) {
                    for (dst, v) in out.iter_mut().zip(sources[idx].iter()) {
                        *dst += *v;
                    }
                }
            }
            Ok(out)
        };

    if mode == "6-track" {
        let wanted = ["vocals", "drums", "bass", "guitar", "piano", "other"];
        for track_type in wanted {
            let source = get_source(track_type)?;
            let out_path = result_dir.join(format!("{}.wav", track_type));
            write_wav(&out_path, source, sr).map_err(|e| format!("保存输出失败: {}", e))?;
            tracks.push(TrackInfo {
                track_type: track_type.to_string(),
                path: out_path.to_string_lossy().to_string(),
                name: get_track_name(track_type),
            });
        }
    } else if mode == "4-track" {
        for track_type in ["vocals", "drums", "bass"] {
            let source = get_source(track_type)?;
            let out_path = result_dir.join(format!("{}.wav", track_type));
            write_wav(&out_path, source, sr).map_err(|e| format!("保存输出失败: {}", e))?;
            tracks.push(TrackInfo {
                track_type: track_type.to_string(),
                path: out_path.to_string_lossy().to_string(),
                name: get_track_name(track_type),
            });
        }

        // "other" may be split into other/guitar/piano for 6s model; merge them.
        let other = sum_sources(&["other", "guitar", "piano"])?;
        let out_path = result_dir.join("other.wav");
        write_wav(&out_path, &other, sr).map_err(|e| format!("保存输出失败: {}", e))?;
        tracks.push(TrackInfo {
            track_type: "other".to_string(),
            path: out_path.to_string_lossy().to_string(),
            name: get_track_name("other"),
        });
    } else {
        let vocals = get_source("vocals")?;
        let no_vocals = if supports_6_track {
            sum_sources(&["drums", "bass", "guitar", "piano", "other"])?
        } else {
            sum_sources(&["drums", "bass", "other"])?
        };

        let vocals_path = result_dir.join("vocals.wav");
        let bgm_path = result_dir.join("no_vocals.wav");
        write_wav(&vocals_path, vocals, sr).map_err(|e| format!("保存输出失败: {}", e))?;
        write_wav(&bgm_path, &no_vocals, sr).map_err(|e| format!("保存输出失败: {}", e))?;

        tracks.push(TrackInfo {
            track_type: "vocals".to_string(),
            path: vocals_path.to_string_lossy().to_string(),
            name: get_track_name("vocals"),
        });
        tracks.push(TrackInfo {
            track_type: "no_vocals".to_string(),
            path: bgm_path.to_string_lossy().to_string(),
            name: get_track_name("no_vocals"),
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
    log_stderr(&format!(
        "[DEBUG] convert_audio_format: input={:?}, output={:?}, ffmpeg={}",
        input_path, output_path, ffmpeg_path
    ));

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

    log_stderr("[DEBUG] Running ffmpeg command...");

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run ffmpeg: {}. Make sure ffmpeg is installed.", e))?;

    log_stderr(&format!("[DEBUG] ffmpeg exit status: {:?}", output.status));

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        log_stderr(&format!("[DEBUG] ffmpeg stderr length: {}", stderr.len()));
        log_stderr(&format!("[DEBUG] ffmpeg stdout length: {}", stdout.len()));
        return Err(format!("Failed to convert to {} format (exit code: {:?})", format, output.status.code()));
    }

    log_stderr("[DEBUG] ffmpeg conversion successful");
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
    base_on_vocals: Option<bool>,
    loop_bgm: Option<bool>,
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

    // Use ffmpeg to merge tracks with volume adjustment.
    // When base_on_vocals is enabled, duration follows vocals length.
    let filter = if base_on_vocals.unwrap_or(false) {
        format!(
            "[1:a]volume={}[vocals];[0:a]volume={}[bgm];[vocals][bgm]amix=inputs=2:duration=first:dropout_transition=2[out]",
            vocals_volume, bgm_volume
        )
    } else {
        format!(
            "[0:a]volume={}[a];[1:a]volume={}[b];[a][b]amix=inputs=2:duration=longest[out]",
            bgm_volume, vocals_volume
        )
    };

    let ffmpeg_path = get_ffmpeg_path(&app);
    let mut cmd = Command::new(&ffmpeg_path);
    cmd.arg("-y"); // Overwrite output
    if loop_bgm.unwrap_or(false) {
        cmd.arg("-stream_loop").arg("-1");
    }
    let status = cmd
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

fn clamp_non_negative(v: f64) -> f64 {
    if v.is_finite() {
        v.max(0.0)
    } else {
        0.0
    }
}

fn clamp_volume(v: f64) -> f64 {
    if v.is_finite() {
        v.max(0.0).min(2.0)
    } else {
        1.0
    }
}

fn run_ffmpeg_with_progress(
    mut cmd: Command,
    window: &Window,
    duration_sec: f64,
    stage: &str,
) -> Result<(), String> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to run ffmpeg: {}. Make sure ffmpeg is installed.", e))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture ffmpeg stdout".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Failed to capture ffmpeg stderr".to_string())?;

    let stderr_handle = std::thread::spawn(move || {
        let mut buf = String::new();
        let mut reader = BufReader::new(stderr);
        let _ = reader.read_to_string(&mut buf);
        buf
    });

    let reader = BufReader::new(stdout);
    let mut last_sent = -1.0_f64;

    for line in reader.lines().flatten() {
        if let Some(rest) = line.strip_prefix("out_time_ms=") {
            // ffmpeg progress uses microseconds in out_time_ms.
            if let Ok(us) = rest.trim().parse::<u64>() {
                let t = (us as f64) / 1_000_000.0;
                let pct = if duration_sec > 0.0 {
                    ((t / duration_sec) * 100.0).min(99.9).max(0.0)
                } else {
                    0.0
                };

                if (pct - last_sent).abs() >= 0.4 {
                    last_sent = pct;
                    let _ = window.emit(
                        "video-mix-progress",
                        ProgressPayload {
                            progress: pct,
                            status: "正在导出 MP4...".to_string(),
                            stage: stage.to_string(),
                        },
                    );
                }
            }
        } else if line.trim() == "progress=end" {
            break;
        }
    }

    let status = child.wait().map_err(|e| e.to_string())?;
    let stderr_out = stderr_handle.join().unwrap_or_default();

    if !status.success() {
        let snippet = stderr_out.lines().rev().take(10).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n");
        if snippet.trim().is_empty() {
            return Err("FFmpeg 处理失败，请检查输入文件与格式。".to_string());
        }
        return Err(format!("FFmpeg 处理失败:\n{}", snippet));
    }

    let _ = window.emit(
        "video-mix-progress",
        ProgressPayload {
            progress: 100.0,
            status: "处理完成！".to_string(),
            stage: stage.to_string(),
        },
    );

    Ok(())
}

#[tauri::command]
async fn mix_bgm_into_video(
    app: tauri::AppHandle,
    window: Window,
    video_path: String,
    bgm_path: String,
    mode: String,
    bgm_volume: f64,
    video_volume: f64,
    bgm_offset_sec: f64,
    loop_bgm: bool,
    fade_in_sec: f64,
    fade_out_sec: f64,
    output_dir: Option<String>,
) -> Result<MergeResult, String> {
    let video_file = PathBuf::from(&video_path);
    let bgm_file = PathBuf::from(&bgm_path);

    if !video_file.exists() {
        return Err(format!("Video file not found: {}", video_path));
    }
    if !bgm_file.exists() {
        return Err(format!("BGM file not found: {}", bgm_path));
    }

    let ffprobe_path = get_ffprobe_path(&app);
    let duration_sec = probe_media_duration_seconds(&ffprobe_path, &video_path)?;
    let has_audio = probe_has_audio_stream(&ffprobe_path, &video_path).unwrap_or(false);

    let bgm_volume = clamp_volume(bgm_volume);
    let video_volume = clamp_volume(video_volume);
    let bgm_offset_sec = clamp_non_negative(bgm_offset_sec);
    let fade_in_sec = clamp_non_negative(fade_in_sec);
    let fade_out_sec = clamp_non_negative(fade_out_sec);

    let effective_mode = if mode == "mix" && has_audio {
        "mix"
    } else {
        "bgmOnly"
    };

    let output_base = if let Some(out) = output_dir {
        PathBuf::from(out)
    } else {
        dirs::document_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
            .join("BGM Extractor Output")
            .join("Video Mix")
    };
    std::fs::create_dir_all(&output_base).map_err(|e| e.to_string())?;

    let stem = video_file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("video");
    let output = output_base.join(format!("{}_with_bgm.mp4", stem));

    let offset_ms = (bgm_offset_sec * 1000.0).round() as i64;
    let fade_out_start = (duration_sec - fade_out_sec).max(0.0);

    let mut bgm_filters: Vec<String> = vec![format!("volume={}", bgm_volume)];
    if offset_ms > 0 {
        bgm_filters.push(format!("adelay={}:all=1", offset_ms));
    }
    bgm_filters.push(format!("atrim=0:duration={}", duration_sec));
    if fade_in_sec > 0.0 {
        bgm_filters.push(format!("afade=t=in:st={}:d={}", bgm_offset_sec, fade_in_sec));
    }
    if fade_out_sec > 0.0 {
        bgm_filters.push(format!(
            "afade=t=out:st={}:d={}",
            fade_out_start, fade_out_sec
        ));
    }
    bgm_filters.push("asetpts=N/SR/TB".to_string());

    let filter = if effective_mode == "mix" {
        format!(
            "[0:a]volume={},atrim=0:duration={},asetpts=N/SR/TB[orig];[1:a]{}[bgm];[orig][bgm]amix=inputs=2:duration=longest:dropout_transition=2,atrim=0:duration={}[aout]",
            video_volume,
            duration_sec,
            bgm_filters.join(","),
            duration_sec
        )
    } else {
        format!("[1:a]{}[aout]", bgm_filters.join(","))
    };

    let ffmpeg_path = get_ffmpeg_path(&app);

    let _ = window.emit(
        "video-mix-progress",
        ProgressPayload {
            progress: 0.0,
            status: "正在准备 FFmpeg...".to_string(),
            stage: "init".to_string(),
        },
    );

    let window_copy = window.clone();
    let ffmpeg_path_task = ffmpeg_path.clone();
    let video_path_task = video_path.clone();
    let bgm_path_task = bgm_path.clone();
    let filter_task = filter.clone();
    let output_task = output.clone();
    let loop_bgm_task = loop_bgm;
    let duration_task = duration_sec;

    tauri::async_runtime::spawn_blocking(move || {
        let build_cmd = |copy_video: bool| {
            let mut cmd = Command::new(&ffmpeg_path_task);
            cmd.arg("-y")
                .arg("-hide_banner")
                .arg("-v")
                .arg("error")
                .arg("-progress")
                .arg("pipe:1")
                .arg("-nostats")
                .arg("-i")
                .arg(&video_path_task);

            if loop_bgm_task {
                cmd.arg("-stream_loop").arg("-1");
            }

            cmd.arg("-i")
                .arg(&bgm_path_task)
                .arg("-filter_complex")
                .arg(&filter_task)
                .arg("-map")
                .arg("0:v:0")
                .arg("-map")
                .arg("[aout]")
                .arg("-c:a")
                .arg("aac")
                .arg("-b:a")
                .arg("192k")
                .arg("-ac")
                .arg("2")
                .arg("-movflags")
                .arg("+faststart");

            if copy_video {
                cmd.arg("-c:v").arg("copy");
            } else {
                cmd.arg("-c:v")
                    .arg("libx264")
                    .arg("-preset")
                    .arg("veryfast")
                    .arg("-crf")
                    .arg("20");
            }

            cmd.arg(&output_task);
            cmd
        };

        let cmd = build_cmd(true);
        if let Err(e) = run_ffmpeg_with_progress(cmd, &window_copy, duration_task, "mixing") {
            let _ = window_copy.emit(
                "video-mix-progress",
                ProgressPayload {
                    progress: 0.0,
                    status: "视频流拷贝失败，正在回退为重编码...".to_string(),
                    stage: "retry".to_string(),
                },
            );
            let cmd2 = build_cmd(false);
            run_ffmpeg_with_progress(cmd2, &window_copy, duration_task, "mixing")
                .map_err(|e2| format!("{}\n{}", e, e2))?;
        }
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| e.to_string())??;

    Ok(MergeResult {
        output_path: output.to_string_lossy().to_string(),
    })
}

#[tauri::command]
async fn save_ref_audio(audio_data: Vec<u8>) -> Result<String, String> {
    let mut tmp_path = std::env::temp_dir();
    let uniq = format!(
        "ref_audio_{}_{}.wav",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );
    tmp_path.push(uniq);
    std::fs::write(&tmp_path, &audio_data).map_err(|e| format!("保存参考音频失败: {}", e))?;
    Ok(tmp_path.to_string_lossy().to_string())
}

struct MicRecordingState {
    child: Child,
    output_path: PathBuf,
}

static MIC_RECORDING: Mutex<Option<MicRecordingState>> = Mutex::new(None);


#[tauri::command]
async fn start_mic_recording(app: tauri::AppHandle, max_seconds: Option<u32>) -> Result<(), String> {
    let mut guard = MIC_RECORDING.lock().map_err(|e| format!("锁定录音状态失败: {}", e))?;
    if guard.is_some() {
        return Err("已有录音正在进行".to_string());
    }

    let ffmpeg_path = get_ffmpeg_path(&app);
    let max_sec = max_seconds.unwrap_or(15).clamp(1, 60);

    let mut tmp_path = std::env::temp_dir();
    let uniq = format!(
        "mic_rec_{}_{}.wav",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );
    tmp_path.push(uniq);

    let mut cmd = Command::new(&ffmpeg_path);
    if cfg!(target_os = "macos") {
        cmd.args(["-f", "avfoundation", "-i", ":0"]);
    } else if cfg!(target_os = "linux") {
        cmd.args(["-f", "pulse", "-i", "default"]);
    } else {
        cmd.args(["-f", "dshow", "-i", "audio=default"]);
    }
    // 22050Hz mono, highpass 80Hz to cut low-freq rumble, light noise gate
    cmd.args(["-ac", "1", "-ar", "22050"])
        .args(["-af", "highpass=f=80,afftdn=nf=-20"])
        .arg("-t").arg(format!("{}", max_sec))
        .arg("-y")
        .arg(&tmp_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = cmd.spawn().map_err(|e| format!("启动录音失败: {}", e))?;
    *guard = Some(MicRecordingState {
        child,
        output_path: tmp_path,
    });
    Ok(())
}

#[tauri::command]
async fn stop_mic_recording() -> Result<String, String> {
    let mut guard = MIC_RECORDING.lock().map_err(|e| format!("锁定录音状态失败: {}", e))?;
    let state = guard.take().ok_or_else(|| "没有正在进行的录音".to_string())?;
    let MicRecordingState { mut child, output_path } = state;

    // Send 'q' to ffmpeg stdin for graceful stop
    if let Some(ref mut stdin) = child.stdin {
        let _ = stdin.write_all(b"q");
    }
    // Wait briefly for graceful exit, then force kill
    let wait_result = std::thread::spawn(move || {
        child.wait()
    }).join();

    match wait_result {
        Ok(Ok(_)) => {}
        _ => {
            // Process already exited or join failed, that's fine
        }
    }

    if output_path.exists() && std::fs::metadata(&output_path).map(|m| m.len() > 44).unwrap_or(false) {
        Ok(output_path.to_string_lossy().to_string())
    } else {
        Err("录音文件为空或未生成，请检查麦克风权限".to_string())
    }
}

#[tauri::command]
async fn synthesize_kokoro_tts(
    app: tauri::AppHandle,
    text: String,
    voice: String,
    language: String,
    speed: f64,
    output_dir: Option<String>,
    _tts_engine: Option<String>,
    _ref_audio_path: Option<String>,
    _ref_text: Option<String>,
) -> Result<MergeResult, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("文本不能为空".to_string());
    }
    ensure_ort_initialized(&app)?;
    let model_path_opt = get_kokoro_model_path(&app);
    let voices_path_opt = get_kokoro_voices_path(&app);

    let clamped_speed = if speed.is_finite() {
        speed.max(0.6).min(1.6)
    } else {
        1.0
    };

    let output_base = if let Some(out) = output_dir {
        PathBuf::from(out)
    } else {
        dirs::document_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
            .join("BGM Extractor Output")
            .join("TTS")
    };
    std::fs::create_dir_all(&output_base).map_err(|e| e.to_string())?;

    let ts_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let output_path = output_base.join(format!(
        "kokoro_tts_{}_{}.wav",
        std::process::id(),
        ts_ms
    ));

    let normalized_lang = normalize_tts_language(&language);

    let output_path_str = output_path.to_string_lossy().to_string();
    let text_owned = text.to_string();
    let voice_owned = if voice.trim().is_empty() {
        "af_heart".to_string()
    } else {
        voice.trim().to_string()
    };

    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let run = || -> Result<(), String> {
            let segments = split_text_for_tts(&text_owned, 80, 220);
            if segments.is_empty() {
                return Err("文本不能为空".to_string());
            }
            let mut merged_audio: Vec<f32> = Vec::new();
            let (model_path, voices_path) =
                ensure_kokoro_local_assets(model_path_opt, voices_path_opt)?;
            let mut cache = KOKORO_TTS_CACHE.lock().map_err(|e| format!("锁定 TTS 缓存失败: {}", e))?;
            if cache.is_none() {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| format!("初始化 TTS 运行时失败: {}", e))?;
                let tts = runtime.block_on(TTSKoko::from_paths(
                    model_path.to_string_lossy().as_ref(),
                    voices_path.to_string_lossy().as_ref(),
                ));
                *cache = Some(tts);
            }
            let tts = cache.as_ref().unwrap();
            let sr = tts.sample_rate();
            let gap_samples = ((sr as f32) * 0.06_f32) as usize;
            for (idx, seg) in segments.iter().enumerate() {
                let audio = tts
                    .tts_raw_audio(
                        seg,
                        &normalized_lang,
                        &voice_owned,
                        clamped_speed as f32,
                        None,
                        false,
                        true,
                        false,
                    )
                    .map_err(|e| format!("Kokoro 分段合成失败（第 {} 段）: {}", idx + 1, e))?;
                merged_audio.extend_from_slice(&audio);
                if idx + 1 < segments.len() && gap_samples > 0 {
                    merged_audio.extend(std::iter::repeat_n(0.0_f32, gap_samples));
                }
            }
            write_stereo_wav_from_mono(&PathBuf::from(&output_path_str), &merged_audio, sr)?;
            Ok(())
        };

        std::panic::catch_unwind(std::panic::AssertUnwindSafe(run))
            .unwrap_or_else(|_| Err("TTS 初始化失败，请检查模型文件和依赖".to_string()))
    })
    .await
    .map_err(|e| format!("TTS 任务执行失败: {}", e))??;

    if !output_path.exists() {
        return Err("TTS 合成未生成音频文件".to_string());
    }

    Ok(MergeResult {
        output_path: output_path.to_string_lossy().to_string(),
    })
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct BatchTtsItem {
    text: String,
    voice: String,
    #[serde(default)]
    language: String,
    #[serde(default = "default_speed")]
    speed: f64,
    #[serde(default)]
    ref_audio_path: Option<String>,
    #[serde(default)]
    ref_text: Option<String>,
}

fn default_speed() -> f64 { 1.0 }

#[derive(Serialize)]
struct BatchTtsResult {
    output_paths: Vec<String>,
}

#[tauri::command]
async fn synthesize_tts_batch(
    app: tauri::AppHandle,
    items: Vec<BatchTtsItem>,
    _tts_engine: Option<String>,
    output_dir: Option<String>,
) -> Result<BatchTtsResult, String> {
    if items.is_empty() {
        return Err("批量合成列表为空".to_string());
    }

    let output_base = if let Some(out) = output_dir {
        PathBuf::from(out)
    } else {
        dirs::document_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
            .join("BGM Extractor Output")
            .join("TTS")
    };
    std::fs::create_dir_all(&output_base).map_err(|e| e.to_string())?;

    let model_path_opt = {
        ensure_ort_initialized(&app)?;
        get_kokoro_model_path(&app)
    };
    let voices_path_opt = get_kokoro_voices_path(&app);
    let output_base_owned = output_base;
    let items_owned = items;

    let output_paths = tauri::async_runtime::spawn_blocking(move || -> Result<Vec<String>, String> {
        let run = || -> Result<Vec<String>, String> {
            let (model_path, voices_path) =
                ensure_kokoro_local_assets(model_path_opt, voices_path_opt)?;
            let mut cache = KOKORO_TTS_CACHE.lock().map_err(|e| format!("锁定 TTS 缓存失败: {}", e))?;
            if cache.is_none() {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| format!("初始化 TTS 运行时失败: {}", e))?;
                let tts = runtime.block_on(TTSKoko::from_paths(
                    model_path.to_string_lossy().as_ref(),
                    voices_path.to_string_lossy().as_ref(),
                ));
                *cache = Some(tts);
            }
            let tts = cache.as_ref().unwrap();
            let sr = tts.sample_rate();
            let mut paths = Vec::with_capacity(items_owned.len());

            for (idx, item) in items_owned.iter().enumerate() {
                let text = item.text.trim();
                if text.is_empty() {
                    return Err(format!("第 {} 项文本为空", idx + 1));
                }
                let lang = normalize_tts_language(&item.language);
                let voice = if item.voice.trim().is_empty() { "af_heart".to_string() } else { item.voice.trim().to_string() };
                let speed = if item.speed.is_finite() { item.speed.max(0.6).min(1.6) } else { 1.0 };

                let segments = split_text_for_tts(text, 80, 220);
                let mut merged_audio: Vec<f32> = Vec::new();
                let gap_samples = ((sr as f32) * 0.06_f32) as usize;
                for (seg_idx, seg) in segments.iter().enumerate() {
                    let audio = tts
                        .tts_raw_audio(seg, &lang, &voice, speed as f32, None, false, true, false)
                        .map_err(|e| format!("第 {} 项分段 {} 合成失败: {}", idx + 1, seg_idx + 1, e))?;
                    merged_audio.extend_from_slice(&audio);
                    if seg_idx + 1 < segments.len() && gap_samples > 0 {
                        merged_audio.extend(std::iter::repeat_n(0.0_f32, gap_samples));
                    }
                }

                let ts_ms = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis();
                let output_path = output_base_owned.join(format!(
                    "kokoro_batch_{}_{}_{}.wav", std::process::id(), ts_ms, idx
                ));
                write_stereo_wav_from_mono(&output_path, &merged_audio, sr)?;
                paths.push(output_path.to_string_lossy().to_string());
            }
            Ok(paths)
        };
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(run))
            .unwrap_or_else(|_| Err("TTS 批量合成失败".to_string()))
    })
    .await
    .map_err(|e| format!("TTS 批量任务执行失败: {}", e))??;

    Ok(BatchTtsResult { output_paths })
}

#[tauri::command]
async fn merge_audio_segments(
    app: tauri::AppHandle,
    input_paths: Vec<String>,
    output_path: Option<String>,
    output_dir: Option<String>,
) -> Result<MergeResult, String> {
    if input_paths.is_empty() {
        return Err("没有可合并的音频片段".to_string());
    }
    for p in &input_paths {
        let path = PathBuf::from(p);
        if !path.exists() {
            return Err(format!("音频片段不存在: {}", p));
        }
    }

    let output = if let Some(path) = output_path {
        PathBuf::from(path)
    } else {
        let output_base = if let Some(dir) = output_dir {
            PathBuf::from(dir)
        } else {
            dirs::document_dir()
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
                .join("BGM Extractor Output")
                .join("Podcast")
        };
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        output_base.join(format!("podcast_voice_merged_{}.wav", ts))
    };

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    if input_paths.len() == 1 {
        std::fs::copy(&input_paths[0], &output)
            .map_err(|e| format!("复制音频片段失败: {}", e))?;
        return Ok(MergeResult {
            output_path: output.to_string_lossy().to_string(),
        });
    }

    let ffmpeg_path = get_ffmpeg_path(&app);
    let mut cmd = Command::new(&ffmpeg_path);
    for p in &input_paths {
        cmd.arg("-i").arg(p);
    }
    let filter = format!("concat=n={}:v=0:a=1[outa]", input_paths.len());
    let output_run = cmd
        .arg("-filter_complex")
        .arg(filter)
        .arg("-map")
        .arg("[outa]")
        .arg("-ac")
        .arg("2")
        .arg("-ar")
        .arg("24000")
        .arg("-c:a")
        .arg("pcm_s16le")
        .arg("-y")
        .arg(&output)
        .output()
        .map_err(|e| format!("Failed to run ffmpeg: {}. Make sure ffmpeg is installed.", e))?;

    if !output_run.status.success() {
        let stderr = String::from_utf8_lossy(&output_run.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output_run.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        return Err(format!("合并片段失败: {}", detail));
    }

    if !output.exists() {
        return Err("合并音频未生成输出文件".to_string());
    }

    Ok(MergeResult {
        output_path: output.to_string_lossy().to_string(),
    })
}

#[tauri::command]
async fn generate_podcast_script_preview(
    window: Window,
    topic: String,
    style: Option<String>,
    language: String,
    roles: Vec<PodcastRole>,
    speed: f64,
    pause_ms: Option<u32>,
    output_dir: Option<String>,
    llm_api_base: Option<String>,
    llm_api_key: Option<String>,
    llm_model: Option<String>,
) -> Result<PodcastScriptPreviewResult, String> {
    let topic = topic.trim();
    if topic.is_empty() {
        return Err("主题不能为空".to_string());
    }
    if roles.len() < 2 {
        return Err("至少需要 2 个角色".to_string());
    }
    if roles.iter().any(|r| r.name.trim().is_empty() || r.voice.trim().is_empty()) {
        return Err("角色名称和音色不能为空".to_string());
    }

    let clamped_speed = if speed.is_finite() {
        speed.max(0.6).min(1.6)
    } else {
        1.0
    };
    let pause_ms = pause_ms.unwrap_or(220).clamp(0, 4000);
    let style = style.unwrap_or_else(|| {
        "Practical, insightful, and conversational with clear takeaways".to_string()
    });
    let target_duration_seconds = extract_target_duration_seconds(topic, &style);
    let normalized_lang = normalize_tts_language(&language);

    let output_base = if let Some(out) = output_dir {
        PathBuf::from(out)
    } else {
        dirs::document_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
            .join("BGM Extractor Output")
            .join("Podcast")
    };
    std::fs::create_dir_all(&output_base).map_err(|e| e.to_string())?;

    let api_key = llm_api_key
        .or_else(|| std::env::var("OPENAI_API_KEY").ok())
        .unwrap_or_default();
    if api_key.trim().is_empty() {
        return Err("缺少 LLM API Key，请在页面填写或设置 OPENAI_API_KEY".to_string());
    }
    let api_base = llm_api_base
        .or_else(|| std::env::var("OPENAI_BASE_URL").ok())
        .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
    let model = llm_model
        .or_else(|| std::env::var("OPENAI_MODEL").ok())
        .unwrap_or_else(|| "glm-4.7".to_string());

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let script_path = output_base.join(format!("podcast_script_{}.json", ts));
    let script_path_str = script_path.to_string_lossy().to_string();

    let window_for_task = window.clone();
    let topic_owned = topic.to_string();
    let style_owned = style.clone();
    let language_owned = normalized_lang.clone();
    let roles_owned = roles.clone();
    let api_base_owned = api_base.clone();
    let api_key_owned = api_key.clone();
    let model_owned = model.clone();
    let script_path_owned = script_path_str.clone();
    let (lines_out, outline_out, estimated, style_out) =
        tauri::async_runtime::spawn_blocking(move || -> Result<(Vec<PodcastLine>, Vec<PodcastOutlineSection>, f64, String), String> {
            emit_podcast_progress(&window_for_task, 2.0, "init", "Initializing script generation...");
            let (lines, outline) = match generate_podcast_script_with_llm(
                &api_base_owned,
                &api_key_owned,
                &model_owned,
                &topic_owned,
                &style_owned,
                &language_owned,
                clamped_speed,
                pause_ms,
                target_duration_seconds,
                &roles_owned,
                &mut |progress, stage, status| emit_podcast_progress(&window_for_task, progress, &stage, status),
            ) {
                Ok(v) => v,
                Err(e) => {
                    log_stderr(&format!(
                        "[PodcastAgent] LLM generation failed, fallback to template script: {}",
                        e
                    ));
                    (
                        build_fallback_podcast_script(
                            &topic_owned,
                            &style_owned,
                            &language_owned,
                            target_duration_seconds,
                            pause_ms,
                            clamped_speed,
                            &roles_owned,
                        ),
                        build_fallback_outline(&topic_owned, &style_owned, target_duration_seconds),
                    )
                }
            };

            if lines.is_empty() {
                return Err("脚本生成失败：未产出可用内容".to_string());
            }
            let estimated = estimate_script_duration_seconds(&lines, pause_ms, clamped_speed);
            let script_json = serde_json::json!({
                "topic": topic_owned,
                "style": style_owned,
                "language": language_owned,
                "roles": roles_owned,
                "target_duration_seconds": target_duration_seconds,
                "estimated_duration_seconds": estimated,
                "outline": outline,
                "lines": lines,
            });
            std::fs::write(&script_path_owned, serde_json::to_vec_pretty(&script_json).unwrap_or_default())
                .map_err(|e| format!("写入脚本文件失败: {}", e))?;
            emit_podcast_progress(&window_for_task, 100.0, "done", "Script generation completed");

            let lines_out: Vec<PodcastLine> = script_json
                .get("lines")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            let outline_out: Vec<PodcastOutlineSection> = script_json
                .get("outline")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            let style_out = script_json
                .get("style")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            Ok((lines_out, outline_out, estimated, style_out))
        })
        .await
        .map_err(|e| format!("脚本预览任务执行失败: {}", e))??;

    Ok(PodcastScriptPreviewResult {
        script_path: script_path_str,
        lines: lines_out,
        outline: outline_out,
        estimated_duration_seconds: Some(estimated),
        target_duration_seconds,
        style: style_out,
    })
}

#[tauri::command]
async fn generate_podcast_dialogue(
    app: tauri::AppHandle,
    window: Window,
    topic: String,
    style: Option<String>,
    language: String,
    roles: Vec<PodcastRole>,
    speed: f64,
    pause_ms: Option<u32>,
    output_dir: Option<String>,
    llm_api_base: Option<String>,
    llm_api_key: Option<String>,
    llm_model: Option<String>,
    _tts_engine: Option<String>,
) -> Result<PodcastResult, String> {
    let topic = topic.trim();
    if topic.is_empty() {
        return Err("主题不能为空".to_string());
    }
    if roles.len() < 2 {
        return Err("至少需要 2 个角色".to_string());
    }
    if roles.iter().any(|r| r.name.trim().is_empty() || r.voice.trim().is_empty()) {
        return Err("角色名称和音色不能为空".to_string());
    }

    ensure_ort_initialized(&app)?;
    let model_path_opt = get_kokoro_model_path(&app);
    let voices_path_opt = get_kokoro_voices_path(&app);

    let clamped_speed = if speed.is_finite() {
        speed.max(0.6).min(1.6)
    } else {
        1.0
    };
    let pause_ms = pause_ms.unwrap_or(220).clamp(0, 4000);
    let style = style.unwrap_or_else(|| {
        "Practical, insightful, and conversational with clear takeaways".to_string()
    });
    let target_duration_seconds = extract_target_duration_seconds(topic, &style);
    let normalized_lang = normalize_tts_language(&language);
    let output_base = if let Some(out) = output_dir {
        PathBuf::from(out)
    } else {
        dirs::document_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
            .join("BGM Extractor Output")
            .join("Podcast")
    };
    std::fs::create_dir_all(&output_base).map_err(|e| e.to_string())?;

    let api_key = llm_api_key
        .or_else(|| std::env::var("OPENAI_API_KEY").ok())
        .unwrap_or_default();
    if api_key.trim().is_empty() {
        return Err("缺少 LLM API Key，请在页面填写或设置 OPENAI_API_KEY".to_string());
    }
    let api_base = llm_api_base
        .or_else(|| std::env::var("OPENAI_BASE_URL").ok())
        .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
    let model = llm_model
        .or_else(|| std::env::var("OPENAI_MODEL").ok())
        .unwrap_or_else(|| "glm-4.7".to_string());

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let output_path = output_base.join(format!("podcast_dialogue_{}.wav", ts));
    let script_path = output_base.join(format!("podcast_dialogue_{}.json", ts));

    let model_path_owned = model_path_opt;
    let voices_path_owned = voices_path_opt;
    let roles_owned = roles.clone();
    let topic_owned = topic.to_string();
    let style_owned = style;
    let language_owned = normalized_lang;
    let api_key_owned = api_key;
    let api_base_owned = api_base;
    let model_owned = model;
    let ffprobe_path_owned = get_ffprobe_path(&app);
    let output_path_str = output_path.to_string_lossy().to_string();
    let script_path_str = script_path.to_string_lossy().to_string();

    let window_for_task = window.clone();
    let (lines, outline, actual_duration_seconds) = tauri::async_runtime::spawn_blocking(move || -> Result<(Vec<PodcastLine>, Vec<PodcastOutlineSection>, f64), String> {
        emit_podcast_progress(&window_for_task, 2.0, "init", "Initializing podcast generation...");
        let kokoro_cache_guard: Option<std::sync::MutexGuard<'_, Option<TTSKoko>>>;
        let sample_rate: Option<u32>;
        {
            let (model_path, voices_path) =
                ensure_kokoro_local_assets(model_path_owned, voices_path_owned)?;
            let mut cache = KOKORO_TTS_CACHE.lock().map_err(|e| format!("锁定 TTS 缓存失败: {}", e))?;
            if cache.is_none() {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| format!("初始化 TTS 运行时失败: {}", e))?;
                let tts = runtime.block_on(TTSKoko::from_paths(
                    model_path.to_string_lossy().as_ref(),
                    voices_path.to_string_lossy().as_ref(),
                ));
                *cache = Some(tts);
            }
            sample_rate = Some(cache.as_ref().unwrap().sample_rate());
            kokoro_cache_guard = Some(cache);
        }
        let max_audio_attempts = 1usize;
        let mut final_script: Vec<PodcastLine> = Vec::new();
        let mut final_outline: Vec<PodcastOutlineSection> = build_fallback_outline(
            &topic_owned,
            &style_owned,
            target_duration_seconds,
        );
        let mut final_style = style_owned.clone();
        let mut final_duration_seconds = 0.0_f64;
        let llm_endpoint = format!("{}/chat/completions", api_base_owned.trim_end_matches('/'));
        let llm_topup_client = reqwest::blocking::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(12))
            .timeout(std::time::Duration::from_secs(90))
            .build()
            .ok();

        for audio_attempt in 0..max_audio_attempts {
            let style_for_attempt = if let Some(target) = target_duration_seconds {
                if audio_attempt == 0 {
                    style_owned.clone()
                } else {
                    let mins = (target as f64) / 60.0;
                    format!(
                        "{}. This retry must be about {:.1} minutes with richer content, concrete examples, and complete discussion depth.",
                        style_owned, mins
                    )
                }
            } else {
                style_owned.clone()
            };

            let (mut script, outline) = match generate_podcast_script_with_llm(
                &api_base_owned,
                &api_key_owned,
                &model_owned,
                &topic_owned,
                &style_for_attempt,
                &language_owned,
                clamped_speed,
                pause_ms,
                target_duration_seconds,
                &roles_owned,
                &mut |progress, stage, status| {
                    emit_podcast_progress(&window_for_task, progress, &stage, status);
                },
            ) {
                Ok(v) => v,
                Err(e) => {
                    log_stderr(&format!(
                        "[PodcastAgent] LLM generation failed, fallback to template script: {}",
                        e
                    ));
                    (
                        build_fallback_podcast_script(
                            &topic_owned,
                            &style_for_attempt,
                            &language_owned,
                            target_duration_seconds,
                            pause_ms,
                            clamped_speed,
                            &roles_owned,
                        ),
                        build_fallback_outline(&topic_owned, &style_for_attempt, target_duration_seconds),
                    )
                }
            };
            if script.is_empty() {
                return Err("脚本生成失败：LLM 与本地兜底都未产出内容".to_string());
            }

            let mut merged_audio: Vec<f32> = Vec::new();
            let section_probe_path = PathBuf::from(&output_path_str).with_extension("section_probe.wav");
            let mut section_probe_index = 0usize;
            let mut section_cumulative_targets: Vec<u32> = Vec::new();
            let mut section_sum = 0u32;
            for sec in &outline {
                section_sum = section_sum.saturating_add(sec.target_seconds);
                section_cumulative_targets.push(section_sum);
            }
            let total_lines = script.len().max(1);
            for (idx, line) in script.iter().enumerate() {
                let tts_progress = 64.0 + ((idx as f64) / (total_lines as f64)) * 30.0;
                emit_podcast_progress(
                    &window_for_task,
                    tts_progress,
                    &format!("tts:{}:{}", idx + 1, total_lines),
                    format!("Synthesizing line {}/{} ({})", idx + 1, total_lines, line.role),
                );
                let line_segments = split_text_for_tts(&line.text, 50, 180);
                if line_segments.is_empty() {
                    continue;
                }
                for seg in line_segments {
                    let tts = kokoro_cache_guard
                        .as_ref()
                        .and_then(|g| g.as_ref())
                        .ok_or_else(|| "Kokoro 引擎未初始化".to_string())?;
                    let audio = tts
                        .tts_raw_audio(
                            &seg,
                            &language_owned,
                            &line.voice,
                            clamped_speed as f32,
                            None,
                            false,
                            true,
                            false,
                        )
                        .map_err(|e| format!("第 {} 句合成失败（{}）: {}", idx + 1, line.role, e))?;
                    merged_audio.extend_from_slice(&audio);
                }

                let current_sr = sample_rate.unwrap_or(24000);
                let gap_samples = ((current_sr as f32) * (pause_ms as f32 / 1000.0)) as usize;
                if idx + 1 < script.len() && gap_samples > 0 {
                    merged_audio.extend(std::iter::repeat_n(0.0_f32, gap_samples));
                }

                let elapsed_seconds = (merged_audio.len() as f64) / (current_sr as f64);
                if let Some(target) = target_duration_seconds {
                    emit_podcast_progress(
                        &window_for_task,
                        tts_progress,
                        &format!("tts:{}:{}", idx + 1, total_lines),
                        format!(
                            "Synthesizing line {}/{} · elapsed {:.1}s / target {}s",
                            idx + 1,
                            total_lines,
                            elapsed_seconds,
                            target
                        ),
                    );
                }

                while section_probe_index < section_cumulative_targets.len()
                    && elapsed_seconds >= section_cumulative_targets[section_probe_index] as f64
                {
                    let probe_sr = sample_rate.unwrap_or(24000);
                    if write_stereo_wav_from_mono(&section_probe_path, &merged_audio, probe_sr).is_ok() {
                        let section_measured = probe_media_duration_seconds(
                            &ffprobe_path_owned,
                            &section_probe_path.to_string_lossy(),
                        )
                        .unwrap_or(elapsed_seconds);
                        let section_title = outline
                            .get(section_probe_index)
                            .map(|s| s.title.as_str())
                            .unwrap_or("Section");
                        emit_podcast_progress(
                            &window_for_task,
                            tts_progress,
                            &format!("section-check:{}:{}", section_probe_index + 1, section_cumulative_targets.len()),
                            format!(
                                "Section {} \"{}\" reached · measured {:.1}s",
                                section_probe_index + 1,
                                section_title,
                                section_measured
                            ),
                        );
                    }
                    section_probe_index += 1;
                }
            }

            if let Some(target) = target_duration_seconds {
                let min_allowed = (target as f64) * 0.90;
                let mut elapsed_seconds = (merged_audio.len() as f64) / (sample_rate.unwrap_or(24000) as f64);
                if elapsed_seconds < min_allowed {
                    let mut topup_round = 0usize;
                    while elapsed_seconds < min_allowed && topup_round < 3 {
                        let gap = min_allowed - elapsed_seconds;
                        let section_idx = section_cumulative_targets
                            .iter()
                            .position(|t| elapsed_seconds < (*t as f64))
                            .unwrap_or_else(|| section_cumulative_targets.len().saturating_sub(1));
                        let section = outline.get(section_idx);
                        emit_podcast_progress(
                            &window_for_task,
                            93.0 + (topup_round as f64),
                            "topup",
                            format!(
                                "Audio short by {:.1}s, extending section {}...",
                                gap,
                                section_idx + 1
                            ),
                        );

                        let extra_lines = if let Some(client) = &llm_topup_client {
                            extend_podcast_script_with_llm(
                                client,
                                &llm_endpoint,
                                &api_key_owned,
                                &model_owned,
                                &topic_owned,
                                &style_for_attempt,
                                &language_owned,
                                clamped_speed,
                                pause_ms,
                                min_allowed.ceil() as u32,
                                section.map(|s| s.title.as_str()),
                                section.map(|s| s.objective.as_str()),
                                section.map(|s| s.target_seconds),
                                &roles_owned,
                                &script,
                            )
                            .unwrap_or_default()
                        } else {
                            Vec::new()
                        };

                        if extra_lines.is_empty() {
                            log_stderr("[PodcastAgent] section top-up returned empty lines");
                            break;
                        }

                        for line in &extra_lines {
                            let line_segments = split_text_for_tts(&line.text, 50, 180);
                            if line_segments.is_empty() {
                                continue;
                            }
                            let current_sr = sample_rate.unwrap_or(24000);
                            let gap_samples = ((current_sr as f32) * (pause_ms as f32 / 1000.0)) as usize;
                            if !merged_audio.is_empty() && gap_samples > 0 {
                                merged_audio.extend(std::iter::repeat_n(0.0_f32, gap_samples));
                            }
                            for seg in line_segments {
                                let tts = kokoro_cache_guard
                                    .as_ref()
                                    .and_then(|g| g.as_ref())
                                    .ok_or_else(|| "Kokoro 引擎未初始化".to_string())?;
                                let audio = tts
                                    .tts_raw_audio(
                                        &seg,
                                        &language_owned,
                                        &line.voice,
                                        clamped_speed as f32,
                                        None,
                                        false,
                                        true,
                                        false,
                                    )
                                    .map_err(|e| format!("章节补充句合成失败（{}）: {}", line.role, e))?;
                                merged_audio.extend_from_slice(&audio);
                            }
                        }
                        script.extend(extra_lines);
                        elapsed_seconds = (merged_audio.len() as f64) / (sample_rate.unwrap_or(24000) as f64);
                        topup_round += 1;
                        emit_podcast_progress(
                            &window_for_task,
                            94.0 + (topup_round as f64),
                            "topup",
                            format!(
                                "Section top-up {}/3 complete, elapsed {:.1}s",
                                topup_round,
                                elapsed_seconds
                            ),
                        );
                    }
                }
            }

            let final_sr = sample_rate.unwrap_or(24000);
            write_stereo_wav_from_mono(&PathBuf::from(&output_path_str), &merged_audio, final_sr)?;
            let _ = std::fs::remove_file(&section_probe_path);
            emit_podcast_progress(&window_for_task, 95.0, "duration", "Measuring final audio duration...");
            let measured_duration = probe_media_duration_seconds(&ffprobe_path_owned, &output_path_str)
                .unwrap_or_else(|e| {
                    log_stderr(&format!(
                        "[PodcastAgent] ffprobe duration check failed, fallback to sample-based estimate: {}",
                        e
                    ));
                    (merged_audio.len() as f64) / (final_sr as f64)
                });

            final_duration_seconds = measured_duration;
            final_script = script;
            final_outline = outline;
            final_style = style_for_attempt;

            if let Some(target) = target_duration_seconds {
                let min_allowed = (target as f64) * 0.90;
                if measured_duration < min_allowed {
                    log_stderr(&format!(
                        "[PodcastAgent] Actual duration still short after top-up ({:.1}s < {:.1}s)",
                        measured_duration,
                        min_allowed
                    ));
                }
            }
            break;
        }
        if final_script.is_empty() {
            return Err("脚本生成失败：未产出可用内容".to_string());
        }
        emit_podcast_progress(&window_for_task, 98.0, "finalize", "Saving script and metadata...");
        let script_json = serde_json::json!({
            "topic": topic_owned,
            "style": final_style,
            "language": language_owned,
            "roles": roles_owned,
            "target_duration_seconds": target_duration_seconds,
            "actual_duration_seconds": final_duration_seconds,
            "outline": final_outline,
            "lines": final_script,
        });
        std::fs::write(&script_path_str, serde_json::to_vec_pretty(&script_json).unwrap_or_default())
            .map_err(|e| format!("写入脚本文件失败: {}", e))?;

        let lines_out: Vec<PodcastLine> = script_json
            .get("lines")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        let outline_out: Vec<PodcastOutlineSection> = script_json
            .get("outline")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        emit_podcast_progress(&window_for_task, 100.0, "done", "Podcast generation completed");
        Ok((lines_out, outline_out, final_duration_seconds))
    })
    .await
    .map_err(|e| format!("Podcast 任务执行失败: {}", e))??;

    if !output_path.exists() {
        return Err("Podcast 合成未生成音频文件".to_string());
    }

    Ok(PodcastResult {
        output_path: output_path.to_string_lossy().to_string(),
        script_path: script_path.to_string_lossy().to_string(),
        lines,
        outline,
        actual_duration_seconds: Some(actual_duration_seconds),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|_app| {
            #[cfg(not(debug_assertions))]
            maybe_navigate_to_dev_url(_app.handle());
            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            extract_bgm,
            merge_tracks,
            merge_multi_tracks,
            merge_audio_segments,
            probe_audio_duration,
            mix_bgm_into_video,
            synthesize_kokoro_tts,
            synthesize_tts_batch,
            save_ref_audio,
            start_mic_recording,
            stop_mic_recording,
            generate_podcast_script_preview,
            generate_podcast_dialogue
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
