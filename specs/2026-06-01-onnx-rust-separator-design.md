# Design: ONNX + Rust 推理后端(htdemucs 2 轨)

- **日期**: 2026-06-01
- **分支**: `feat/onnx-rust-separator`
- **目标**: 把 htdemucs 默认 2 轨(人声 + BGM)分离从「调 Python/PyTorch」迁移到「Tauri Rust 后端内用 ONNX Runtime(`ort`)推理」,为 macOS Apple Silicon 上「零外部依赖、双击即用」的独立包打基础。其余模型/模式保留现有 Python 路径作回退。

## 1. 背景与动机

当前 `src-tauri/src/lib.rs` 的 `extract_bgm` 通过 `Command::new("python3")` 调 `script/bgm_extractor.py`,后者用 demucs(PyTorch)的 `get_model`/`apply_model`/`AudioFile`。打包思路是 PyInstaller `--onefile` 把 torch+demucs 打成 sidecar,痛点:包体 ~1.5–2.5GB、onefile 每次启动解压慢、torch 动态库与 macOS 签名/公证冲突。

调研结论(详见会话内 deep-research 报告)确认了可行的轻量化路径:
- htdemucs(Hybrid Transformer)可干净导出 ONNX,无需重训练,质量损失 < 0.1 dB —— 通过把 STFT/iSTFT 改写成实数卷积烘焙进图,规避 ONNX 不支持复数张量这一核心障碍。参考:[adefossez/demucs PR #10](https://github.com/adefossez/demucs/pull/10)、[Mixxx GSoC 2025 博客](https://mixxx.org/news/2025-10-27-gsoc2025-demucs-to-onnx-dhunstack/)。
- Rust `ort`(ONNX Runtime 绑定)在 macOS arm64 有预编译二进制、支持 `coreml` feature 免编译。已有真实项目 [Stemgen](https://github.com/acolombier/stemgen) 用 Rust+ort 跑通导出的 htdemucs。

## 2. 范围(本 PR)

**做:**
- 仅 **htdemucs 默认模型 + 2 轨模式(vocals + no_vocals/BGM)** 走新的 Rust ONNX 路径。
- 输入解码复用 **ffmpeg**(应用本就依赖它,合并功能也需要);输出 stem 在 Rust 内写 16-bit WAV。
- htdemucs ONNX 权重**随包内置离线**(Tauri `bundle.resources`)。
- 保留现有 Python 路径作为**回退**(其它模型、6 轨/4 轨模式、以及 ONNX 路径任何失败时)。

**不做(本 PR 之外):**
- htdemucs_ft / htdemucs_6s / mdx_extra 的 ONNX 化(继续走 Python)。
- 纯 Rust 音频解码(symphonia)—— 仍用 ffmpeg。
- 代码签名 / 公证 / DMG 发布流程。
- 移除 Python/PyInstaller 路径(本 PR 仅"新增并默认优先",不删除回退)。

## 3. 关键决策

| 决策 | 选择 | 理由 |
|---|---|---|
| ONNX 变体 | **PR#10 自包含图**(STFT 烘焙进图) | Rust 侧无需做 FFT,改造面最小,Mixxx/Stemgen 已验证 |
| 推理运行时 | **Rust `ort` 2.x**,download 策略 | arm64 预编译,免编译 ORT;可在 Tauri 后端内直接跑 |
| 加速 EP | **先 CPU EP**,coreml 放 cargo feature | 先保证正确性;CoreML 可能因算子碎片化反而更慢,后续 benchmark 再开 |
| 输入解码 | **ffmpeg** → f32 PCM | 不新增依赖,格式覆盖全 |
| 模型权重存储 | **gitignore + `export_onnx.py` 生成,`tauri:build` 时打进包** | 仓库保持轻量,不引入 Git LFS |
| 质量未达标时 | 以平价为目标;ONNX 路径始终有 Python 回退兜底,不阻塞合入;达标后默认走 ONNX | 即使未完全达标也不退化用户体验 |

## 4. 架构与数据流

### 4.1 `extract_bgm` 分流
```
extract_bgm(model, separation_mode, ...):
  if model == "htdemucs"
     && (separation_mode 解析为 "2-track")
     && onnx 模型存在
     && ONNX 路径未被禁用:
       separator::separate_htdemucs_2track(input, output_dir, &window) ── 新路径
       # 失败时 catch → 落到下面的回退
  else:
       <现有 bundled-binary / python3 回退>   # 保持不变
```
输出目录布局与事件**保持不变**:`<output_dir>/htdemucs/<input_stem>/vocals.wav` 与 `no_vocals.wav`;沿用 `extraction-progress` 事件(`progress`/`status`/`stage`),前端/History/结果页零改动。

**与现有后处理尾段的衔接(评审要点)**:当前 `extract_bgm` 是单体函数——binary/python 的选择在**前段**,而进度解析、退出码检查、输出文件校验、导出格式转换(`lib.rs` 约 398–422)、`ExtractResult` 组装(约 433–450)都在**后段共享**。本 PR 的新分流应置于 binary/python spawn **之前**;ONNX 路径产出同样的 `vocals.wav`/`no_vocals.wav` 后,**汇入同一后段共享逻辑**(复用而非复制格式转换与结果组装)。`separate_htdemucs_2track` 返回的 `OutputPaths` 即作为该后段的输入。这样"前端零改动"才成立。实现计划需明确这一汇入点。

### 4.2 分离管线(精确复刻 `bgm_extractor.py` + demucs `apply_model`)
1. **解码**: `ffmpeg -i <input> -f f32le -acodec pcm_f32le -ac 2 -ar 44100 -` → stdout 读 PCM → 去交织成 `[2][N]`。ffmpeg 路径用现有 `get_ffmpeg_path()`。
2. **归一化**: `ref = wav.mean(axis=0)`; `wav = (wav - ref.mean()) / ref.std()`(脚本第 103–104 行)。保存 `ref_mean`、`ref_std`。
3. **分段 + 加权 overlap-add**(保真关键):
   - 段长 `segment_len = round(model.segment * 44100)`(htdemucs segment≈7.8s)。
   - 步长 `stride = round(segment_len * (1 - overlap))`,`overlap = 0.25`。
   - 过渡权重窗 `weight`(三角/中心加权),逐段输出乘权累加、权重累加,最后相除。
   - 参照 Stemgen `src/demucs.rs` 的实现移植;`shifts` 默认 0(不做随机移位)。
4. **逐块推理**: 每段 pad/trim 到 `segment_len`,组 `ort` 张量 `[1, 2, segment_len]`,`session.run` 得 `[1, 4, 2, segment_len]`。源顺序**假定**为 `["drums","bass","other","vocals"]`(vocals=索引 3)——但这取决于导出图的实际输出顺序,**须在导出后用一次性断言核对**(如比对已知输入的输出能量分布),不可盲信硬编码索引。
5. **拼接 + 反归一化**: 得 `[4][2][N]`;`sources = sources * ref_std + ref_mean`。
6. **2 轨输出**: `vocals = sources[3]`;`no_vocals(BGM) = sum(sources, axis=0) - vocals`(脚本 two-stems 同逻辑)。clip 到 [-1,1]、转 16-bit、写 `vocals.wav` 与 `no_vocals.wav`。
7. **进度**: 按 `已处理段数 / 总段数` 发 `extraction-progress`(比解析 demucs stderr 更平滑)。

## 5. 组件 / 模块

| 文件 | 职责 | 依赖 |
|---|---|---|
| `src-tauri/src/separator/mod.rs` | 对外 `separate_htdemucs_2track(input, output_dir, window) -> Result<OutputPaths, String>`;协调下面各步 | 其余子模块 |
| `src-tauri/src/separator/decode.rs` | spawn ffmpeg 解码 → `Vec<f32>` → `[[f32;2]]` | std::process |
| `src-tauri/src/separator/chunking.rs` | 分段、加权窗、overlap-add(纯函数,可单测) | — |
| `src-tauri/src/separator/model.rs` | 解析 onnx 资源路径、`ort::Session` 初始化、跑单块 | `ort`, `ndarray` |
| `src-tauri/src/separator/wav.rs` | 写 16-bit PCM WAV | `hound` |
| `script/export_onnx.py` | dev 时在 .venv 拉 demucs PR#10、转出 `htdemucs.onnx` 到 `src-tauri/resources/models/` | torch/demucs |

**模型资源解析**(`model.rs`):生产 `resource_dir()/models/htdemucs.onnx`;dev `src-tauri/resources/models/htdemucs.onnx`;沿用现有 binary-resolution 的双路径风格。

**新依赖**(`src-tauri/Cargo.toml`):`ort`(2.x,默认 features 含 download + CPU;`coreml` 为可选 feature)、`hound`、`ndarray`。

**配置**:`tauri.conf.json` 的 `bundle.resources` 加入 `resources/models/htdemucs.onnx`(注:之前移除的 `externalBin` 不恢复;模型走 resources 而非 sidecar)。

**gitignore**:`src-tauri/resources/models/*.onnx`(权重不入库,由 `export_onnx.py` 生成 / 构建时拉取)。

## 6. 接口契约

```rust
// separator/mod.rs
pub struct OutputPaths { pub vocals: PathBuf, pub no_vocals: PathBuf }

pub fn separate_htdemucs_2track(
    input: &Path,
    output_dir: &Path,
    on_progress: impl Fn(f32, &str),   // (progress 0..100, status)
) -> Result<OutputPaths, SeparatorError>;
```
`extract_bgm` 捕获 `SeparatorError` → 记录日志 → 回退 Python 路径。`on_progress` 适配现有 `window.emit("extraction-progress", ...)`。

## 7. 错误处理 / 回退链

1. ONNX 模型文件缺失 / `ort::Session` 初始化失败 → 回退 Python。
2. ffmpeg 解码失败 / 0 采样 → 回退 Python(并保留 ffmpeg stderr 供日志)。
3. 推理过程出错(形状不符、ort 报错)→ 回退 Python。
4. 逃生开关:环境变量 `BGM_DISABLE_ONNX=1` 或设置项,强制走 Python。
5. 所有回退都记录 `eprintln!("[onnx] fallback: <reason>")`,便于诊断。

降级路线(若 PR#10 导出本身失败):改用现成 MDX-Net ONNX 权重(人声/伴奏二分,需 Rust 侧补 STFT),或本 PR 范围缩回"仅打通 ort + 一段 PoC"。

## 8. 测试 / 验收

- **单元**(Rust):`chunking` 的加权窗在重叠区求和≈1、首尾段对齐;`wav` 写出可被 ffmpeg 读回且时长/声道正确。
- **平价验证**(脚本):对同一组测试音频(含早先 `/tmp/bgm-test/sample.wav` + 至少 1 首真实歌曲)分别跑新 Rust 路径与旧 Python 路径,比较 `vocals`/`no_vocals` 的 RMS 差与互相关;目标贴近研究里的 < 0.1 dB(以"无明显接缝/可听差异"为底线)。
- **集成**:`extract_bgm` 在 htdemucs+2-track 命中新路径;在 htdemucs_6s / mdx_extra 命中 Python 回退;在模型缺失时回退。
- **人工**:启动 app,真实歌曲跑 htdemucs 2 轨,听接缝与音质;确认进度条平滑、输出目录与 History 正常。

## 9. 风险

- **分段/overlap-add 保真**:窗函数或步长不对会产生周期性接缝杂音。缓解:移植 Stemgen 参考实现 + 平价验证。
- **PR#10 未合并**:需从该 PR 分支取转换代码,API 可能变。缓解:`export_onnx.py` 固定 commit。
- **CoreML 碎片化**:不支持算子静默回退 CPU 拖慢。缓解:默认 CPU EP,coreml 作 feature + 实测后再开。
- **ort 版本张力**:README 标 ORT 1.26,稳定 RC bundle 的是 1.24。缓解:锁定 `ort` 版本并在 CI/本地核对。
- **包体增大**:.dmg +80–160MB。已接受(换取零依赖)。

## 10. PR 提交切片

1. `export_onnx.py` + resources/gitignore/tauri.conf 模型管线骨架。
2. `separator` 模块 + `ort`/`hound`/`ndarray` 依赖(CPU EP,跑通单块)。
3. 接 `extract_bgm` 分流 + 回退链 + 进度事件。
4. 单测 + 平价验证脚本。
5. 文档(README 构建说明、coreml feature 说明)。
