# BGM Player

> 一款在**本地**运行的桌面应用,用 [Demucs](https://github.com/adefossez/demucs) AI 模型从任意歌曲中提取**背景音乐**——去除人声(或拆分成多个音轨)。

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT"></a>
  <img src="https://img.shields.io/badge/Tauri-2-FFC131?logo=tauri&logoColor=white" alt="Tauri 2">
  <img src="https://img.shields.io/badge/Vue-3-42b883?logo=vuedotjs&logoColor=white" alt="Vue 3">
  <img src="https://img.shields.io/badge/Rust-stable-000000?logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/Demucs-htdemucs-8A2BE2" alt="Demucs">
</p>

<p align="center"><a href="README.md">English</a> · 简体中文</p>

<p align="center"><img src="assets/screenshot.png" alt="BGM Player 截图" width="760"></p>

## 功能特性

- 🎵 **人声 / 背景音乐分离** —— 去掉人声保留伴奏,或将音轨拆分为 2 / 4 / 6 个音轨。
- 🧠 **多种 Demucs 模型** —— `htdemucs`(均衡)、`htdemucs_ft`(最高质量)、`htdemucs_6s`(6 轨)、`mdx_extra`。
- ⚡ **GPU 加速** —— 自动使用 Apple 芯片 **MPS** 与 NVIDIA **CUDA**,无 GPU 时回退到 CPU。
- 📋 **批量队列** —— 拖拽多个文件(MP3、WAV、FLAC、M4A、OGG、AAC)依次处理。
- 🎚️ **音轨合并** —— 按音轨独立调节音量后重新合成并导出。
- 🔊 **内置播放器** —— 试听 BGM / 人声 / 混音,独立音量与波形显示。
- 🗂️ **历史与项目** —— 记录每次提取、快速打开输出目录、用项目组织工作。
- 🎛️ **灵活导出** —— WAV / FLAC(无损)或 MP3 / AAC / M4A / OGG / OPUS,可设置采样率与比特率。
- 🌓 **精致界面** —— 浅色 / 深色 / 跟随系统主题,支持中文与 English。

## 技术栈

| 层 | 技术 |
|------|-------|
| 前端 | Vue 3 · TypeScript · Tailwind CSS · reka-ui |
| 桌面外壳 | Tauri 2(Rust) |
| 音频分离 | Python · [Demucs](https://github.com/adefossez/demucs)(PyTorch) |
| 音频读写与合并 | FFmpeg |

Rust 后端向前端暴露 `extract_bgm` 与 `merge_tracks` 命令。提取在生产环境调用打包好的独立二进制,在开发环境回退到 Python 脚本;合并使用 FFmpeg 完成。

## 环境要求

- **Node.js** ≥ 18 与 npm
- **Rust**(stable)—— `rustup default stable`
- **FFmpeg** 且在 `PATH` 中(合并与音频解码需要)
- **Python 3.10–3.12** 且安装 Demucs(开发环境的提取路径需要,见下)。*注意:过新的 Python(如 3.14)可能还没有 PyTorch 的预编译包。*

## 快速开始(开发)

```bash
# 1. 克隆
git clone https://github.com/JackCaow/bgm-player.git
cd bgm-player

# 2. 前端依赖
npm install

# 3. 给 Demucs 准备 Python 环境(开发提取路径使用)
#    推荐用 uv;普通 python -m venv 也可以。
uv venv --python 3.11 .venv
uv pip install --python .venv/bin/python demucs scipy numpy

# 4. 启动桌面应用(脚本会把 venv 的 python 放到 PATH,使提取可用)
bash script/dev.sh
```

`script/dev.sh` 是一层简单封装:把 `.venv/bin` 加到 `PATH` 最前(让应用里的 `python3` 解析到带 Demucs 的解释器),设置 `PYTORCH_ENABLE_MPS_FALLBACK=1`,然后运行 `npm run tauri:dev`。

如果你的系统级 Python 已经装好 Demucs,也可以直接运行 `npm run tauri:dev`。

> 首次提取时,Demucs 会下载所选模型(几十到几百 MB)并缓存。

## 构建

```bash
npm run tauri:build      # 原生安装包(macOS 上为 .app / .dmg)
```

若要打包成完全自包含的安装包,应用可以携带独立的 sidecar 二进制(`bgm-extractor`、`ffmpeg`、`ffprobe`),放在 `src-tauri/binaries/` 下:用 `script/build_binary.py`(PyInstaller)构建提取器二进制并加入 FFmpeg/FFprobe,然后在 `src-tauri/tauri.conf.json` 中重新启用 `externalBin`。否则应用在运行时依赖系统 Python(已装 Demucs)与系统 FFmpeg。

## 模型

| 模型 | 音轨数 | 说明 |
|-------|-------|-------|
| `htdemucs` | 4(人声/鼓/贝斯/其他) | 推荐 —— 速度与质量均衡 |
| `htdemucs_ft` | 4 | 微调版 —— 质量最高,较慢 |
| `htdemucs_6s` | 6(+ 吉他/钢琴) | 六轨分离 |
| `mdx_extra` | 4 | 基于 MDX 的备选模型 |

## 目录结构

```
src/                 Vue 3 前端(组件、composables、i18n、工具)
src-tauri/           Rust 后端(Tauri 命令在 src/lib.rs)
script/              Python 提取脚本 + 开发启动脚本(dev.sh)
assets/              README 使用的截图等静态资源
```

更多规划见 [ROADMAP.md](ROADMAP.md)。

## 贡献

欢迎提交 Issue 与 Pull Request。项目未强制配置 linter/formatter,请与现有代码风格保持一致(2 空格缩进、TS/JS 使用双引号、Vue 组件 PascalCase 命名、composables 采用 `useX.ts`)。

## 致谢

- [Demucs](https://github.com/adefossez/demucs) —— 驱动音频分离的音源分离模型。
- [Tauri](https://tauri.app/) —— 轻量的桌面应用框架。
- [FFmpeg](https://ffmpeg.org/) —— 音频解码与合并。

## 许可证

[MIT](LICENSE) © 2026 JackCaow
