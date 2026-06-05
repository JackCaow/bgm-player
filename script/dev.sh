#!/usr/bin/env bash
# 启动 BGM Extractor 开发版（Tauri）
# 自动把项目内的 Python venv 放到 PATH 最前，使应用调用的 python3 带有 demucs/torch。
# 用法: bash script/dev.sh
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VENV_BIN="$PROJECT_ROOT/.venv/bin"

if [ ! -x "$VENV_BIN/python3" ]; then
  echo "未找到 venv：$VENV_BIN/python3"
  echo "请先创建并安装依赖："
  echo "  uv venv --python 3.11 .venv"
  echo "  uv pip install --python .venv/bin/python demucs scipy numpy"
  exit 1
fi

# 确保打包用的静态 ffmpeg sidecar 存在(externalBin 在 dev/build 都会校验其存在)
FFMPEG_BIN="$PROJECT_ROOT/src-tauri/binaries/ffmpeg-aarch64-apple-darwin"
if [ ! -x "$FFMPEG_BIN" ]; then
  echo "未找到打包用 ffmpeg sidecar,正在从 release 下载(约 45MB)..."
  mkdir -p "$PROJECT_ROOT/src-tauri/binaries"
  curl -fL "https://github.com/JackCaow/bgm-player/releases/download/models/ffmpeg-aarch64-apple-darwin" \
    -o "$FFMPEG_BIN" && chmod +x "$FFMPEG_BIN" \
    || { echo "下载 ffmpeg 失败,请手动放置到: $FFMPEG_BIN"; exit 1; }
fi

# demucs 在 macOS 上遇到不支持的算子时回退到 CPU
export PYTORCH_ENABLE_MPS_FALLBACK=1
# venv 优先：应用里的 Command::new("python3") 会解析到带 demucs 的解释器
export PATH="$VENV_BIN:$PATH"
# Rust 工具链
[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"

echo "python3 -> $(command -v python3)"
cd "$PROJECT_ROOT"
exec npm run tauri:dev
