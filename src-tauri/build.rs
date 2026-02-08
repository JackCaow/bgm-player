fn main() {
    let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    // Ensure relative paths in `tauri.conf.json` are resolved from `src-tauri/`.
    let _ = std::env::set_current_dir(&manifest_dir);

    // Copy ONNX models into src-tauri so Tauri can bundle them as resources.
    // (Tauri bundling doesn't reliably support resource globs that traverse `..`.)
    let project_root = manifest_dir.parent().unwrap_or(&manifest_dir);
    let src_models_dir = project_root.join("onnx_models");
    let dst_models_dir = manifest_dir.join("onnx_models");

    let _ = std::fs::create_dir_all(&dst_models_dir);

    if src_models_dir.exists() {
        for filename in [
            "htdemucs_core.onnx",
            "htdemucs_config.json",
            "htdemucs_ft_core_0.onnx",
            "htdemucs_ft_core_1.onnx",
            "htdemucs_ft_core_2.onnx",
            "htdemucs_ft_core_3.onnx",
            "htdemucs_ft_config.json",
            "htdemucs_6s_core.onnx",
            "htdemucs_6s_config.json",
        ] {
            let src = src_models_dir.join(filename);
            let dst = dst_models_dir.join(filename);

            println!("cargo:rerun-if-changed={}", src.display());

            if !src.exists() {
                continue;
            }

            let should_copy = match (std::fs::metadata(&src), std::fs::metadata(&dst)) {
                (Ok(src_meta), Ok(dst_meta)) => src_meta.len() != dst_meta.len(),
                (Ok(_), Err(_)) => true,
                _ => false,
            };

            if should_copy {
                let _ = std::fs::copy(&src, &dst);
            }
        }
    }

    tauri_build::build()
}
