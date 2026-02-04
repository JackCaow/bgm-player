# Demucs 音频分离技术调研报告

## 1. 概述

Demucs 是 Meta (Facebook) AI Research 开发的开源音频分离工具，基于深度学习技术，可以将音乐分离为不同的音轨（人声、鼓、贝斯、其他乐器等）。

- **GitHub**: https://github.com/facebookresearch/demucs
- **论文**: Hybrid Transformers for Music Source Separation (2022)
- **许可证**: MIT License

## 2. 可用模型

### 2.1 模型列表

| 模型名称 | 类型 | 分离轨道 | 子模型数 | 说明 |
|----------|------|----------|----------|------|
| htdemucs | Hybrid Transformer | 4轨 | 1 | 默认模型，速度与质量平衡 |
| htdemucs_ft | Hybrid Transformer | 4轨 | 4 | 微调版，质量最佳 |
| htdemucs_6s | Hybrid Transformer | 6轨 | 1 | 支持吉他和钢琴分离 |
| hdemucs_mmi | Hybrid Demucs | 4轨 | 1 | 音乐混合训练版 |
| mdx | MDX | 4轨 | 4 | MDX 比赛模型 |
| mdx_extra | MDX | 4轨 | 4 | MDX 增强版 |
| mdx_q | MDX | 4轨 | 4 | MDX 量化版（更快） |
| mdx_extra_q | MDX | 4轨 | 4 | MDX 增强量化版 |

### 2.2 分离轨道说明

**4轨模型输出：**
- `drums` - 鼓/打击乐
- `bass` - 贝斯
- `other` - 其他乐器
- `vocals` - 人声

**6轨模型输出 (htdemucs_6s)：**
- `drums` - 鼓/打击乐
- `bass` - 贝斯
- `other` - 其他乐器
- `vocals` - 人声
- `guitar` - 吉他
- `piano` - 钢琴

## 3. 性能测试

### 3.1 测试环境

| 项目 | 配置 |
|------|------|
| 设备 | MacBook (Apple M4) |
| 内存 | 24 GB |
| CPU 核心 | 10 核 (4P + 6E) |
| 加速方式 | MPS (Metal Performance Shaders) |
| 测试音频 | 周杰伦 - 稻香 (223.5秒 / 3.7分钟) |

### 3.2 处理速度对比

| 模型 | 处理耗时 | 实时倍率 | 相对速度 |
|------|----------|----------|----------|
| **htdemucs** | 34.7s | 0.16x | ⭐⭐⭐⭐⭐ 最快 |
| mdx | 67.0s | 0.30x | ⭐⭐⭐ |
| mdx_extra | 79.1s | 0.35x | ⭐⭐ |
| htdemucs_ft | 179.8s | 0.80x | ⭐ 最慢 |

> 实时倍率：处理时间 / 音频时长，越小越快

### 3.3 CPU vs GPU 对比

| 设备 | htdemucs 耗时 | 提升幅度 |
|------|---------------|----------|
| CPU | 65.6s | 基准 |
| MPS (M4 GPU) | 34.7s | **快 47%** |

## 4. 模型选择建议

### 4.1 按使用场景

| 场景 | 推荐模型 | 理由 |
|------|----------|------|
| 日常使用 | htdemucs | 速度快，质量好，平衡之选 |
| 追求质量 | htdemucs_ft | 质量最佳，但速度慢 4-5 倍 |
| 批量处理 | htdemucs | 速度优先 |
| 提取吉他/钢琴 | htdemucs_6s | 唯一支持 6 轨分离 |
| 资源受限 | mdx_q | 量化模型，内存占用小 |

### 4.2 质量评估

根据 MDX 2023 比赛和社区反馈：

| 模型 | 人声分离质量 | 伴奏质量 | 综合评分 |
|------|--------------|----------|----------|
| htdemucs_ft | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | 9.5/10 |
| mdx_extra | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | 9.0/10 |
| htdemucs | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | 8.5/10 |
| mdx | ⭐⭐⭐⭐ | ⭐⭐⭐ | 8.0/10 |

## 5. 优化技巧

### 5.1 加速方法

1. **启用 GPU 加速**
   ```bash
   # macOS (Apple Silicon)
   demucs -d mps input.mp3

   # NVIDIA GPU
   demucs -d cuda input.mp3
   ```

2. **使用 --two-stems 参数**
   ```bash
   # 只分离人声和伴奏，速度更快
   demucs --two-stems=vocals input.mp3
   ```

3. **调整 segment 大小**
   ```bash
   # 减小 segment 可降低显存占用
   demucs --segment 10 input.mp3
   ```

4. **使用 shifts 参数**
   ```bash
   # shifts=0 最快，shifts=10 质量最好
   demucs --shifts 0 input.mp3  # 快速模式
   demucs --shifts 5 input.mp3  # 平衡模式
   ```

### 5.2 内存优化

| 参数 | 作用 | 建议值 |
|------|------|--------|
| --segment | 分段大小 | 8GB 显存用默认，4GB 用 10 |
| --overlap | 重叠比例 | 默认 0.25 |
| --no-split | 不分段 | 仅大显存使用 |

## 6. 使用示例

### 6.1 基本用法

```bash
# 提取伴奏（去人声）
demucs --two-stems=vocals -d mps -n htdemucs input.mp3

# 输出文件位置
# output/htdemucs/input/vocals.wav    # 人声
# output/htdemucs/input/no_vocals.wav # 伴奏
```

### 6.2 完整分离

```bash
# 分离为 4 轨
demucs -d mps -n htdemucs input.mp3

# 输出：drums.wav, bass.wav, other.wav, vocals.wav
```

### 6.3 6 轨分离

```bash
# 分离为 6 轨（含吉他、钢琴）
demucs -d mps -n htdemucs_6s input.mp3

# 输出：drums.wav, bass.wav, other.wav, vocals.wav, guitar.wav, piano.wav
```

## 7. 技术架构

### 7.1 Hybrid Transformer 架构

```
输入音频
    ↓
┌─────────────────────────────────────┐
│  时域编码器 (Temporal Encoder)       │
│  - 1D 卷积层                         │
│  - 提取时域特征                      │
└─────────────────────────────────────┘
    ↓
┌─────────────────────────────────────┐
│  频域编码器 (Spectral Encoder)       │
│  - STFT 变换                         │
│  - 2D 卷积层                         │
└─────────────────────────────────────┘
    ↓
┌─────────────────────────────────────┐
│  Transformer 层                      │
│  - 自注意力机制                      │
│  - 跨域特征融合                      │
└─────────────────────────────────────┘
    ↓
┌─────────────────────────────────────┐
│  解码器 (Decoder)                    │
│  - 分离各音轨                        │
└─────────────────────────────────────┘
    ↓
输出：drums, bass, other, vocals
```

### 7.2 模型参数量

| 模型 | 参数量 | 模型文件大小 |
|------|--------|--------------|
| htdemucs | ~26M | 80 MB |
| htdemucs_ft | ~104M (4×26M) | 320 MB |
| htdemucs_6s | ~18M | 52 MB |
| mdx | ~160M | 640 MB |

## 8. 竞品对比

| 工具 | 开源 | 质量 | 速度 | 易用性 |
|------|------|------|------|--------|
| **Demucs** | ✅ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ |
| Spleeter | ✅ | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| Ultimate Vocal Remover | ✅ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐ |
| LALAL.AI | ❌ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| iZotope RX | ❌ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐ |

## 9. 结论与建议

### 9.1 推荐配置

对于本项目（get-bgm），推荐使用：

- **模型**: `htdemucs`
- **设备**: `mps` (Apple Silicon GPU)
- **参数**: `--two-stems=vocals`

预期性能：
- 处理 1 分钟音频约需 **9 秒**
- 处理 4 分钟歌曲约需 **35 秒**

### 9.2 未来优化方向

1. **批量处理**: 实现多文件并行处理
2. **模型缓存**: 预加载模型减少启动时间
3. **增量处理**: 支持实时流式处理
4. **质量选项**: 提供快速/标准/高质量三档选择

---

*报告生成时间: 2026-02-02*
*测试设备: Apple M4 / 24GB RAM*
