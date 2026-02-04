# 音频格式支持文档

## 概述

BGM Extractor 现在支持 7 种音频输出格式,满足不同场景的需求。

## 支持的格式

### 无损格式

#### 1. WAV (Waveform Audio File Format)
- **编码器**: PCM (pcm_s16le)
- **特点**:
  - 完全无损,音质最佳
  - 文件体积最大
  - 兼容性最好
- **适用场景**:
  - 专业音频制作
  - 需要后期编辑
  - 对音质要求极高
- **文件大小**: ~10MB/分钟 (立体声, 44.1kHz, 16-bit)

#### 2. FLAC (Free Lossless Audio Codec)
- **编码器**: FLAC
- **特点**:
  - 无损压缩
  - 文件体积约为 WAV 的 50-60%
  - 开源免费
- **适用场景**:
  - 音乐收藏
  - 需要无损但节省空间
  - 支持元数据标签
- **文件大小**: ~5-6MB/分钟 (立体声, 44.1kHz)

### 有损格式

#### 3. MP3 (MPEG Audio Layer III)
- **编码器**: libmp3lame
- **特点**:
  - 最广泛支持的格式
  - 良好的压缩率
  - 成熟稳定
- **比特率选项**: 128/192/256/320 kbps
- **适用场景**:
  - 日常听歌
  - 移动设备
  - 网络分享
- **文件大小**:
  - 128 kbps: ~1MB/分钟
  - 192 kbps: ~1.5MB/分钟
  - 256 kbps: ~2MB/分钟
  - 320 kbps: ~2.5MB/分钟

#### 4. AAC (Advanced Audio Coding)
- **编码器**: AAC
- **特点**:
  - 比 MP3 更高效
  - 相同比特率下音质更好
  - Apple 设备原生支持
- **比特率选项**: 128/192/256/320 kbps
- **适用场景**:
  - iOS/macOS 设备
  - 流媒体服务
  - 视频配音
- **文件大小**: 与 MP3 相似,但音质更好

#### 5. M4A (MPEG-4 Audio)
- **编码器**: AAC (in MP4 container)
- **特点**:
  - AAC 编码的 MP4 容器
  - Apple 生态系统标准格式
  - 支持元数据和封面
- **比特率选项**: 128/192/256/320 kbps
- **适用场景**:
  - iTunes/Apple Music
  - iPhone/iPad
  - macOS 应用
- **文件大小**: 与 AAC 相同

#### 6. OGG (Ogg Vorbis)
- **编码器**: libvorbis
- **特点**:
  - 开源免费
  - 优秀的音质/体积比
  - 支持可变比特率 (VBR)
- **比特率选项**: 128/192/256/320 kbps 或质量等级 (q:a 6 ≈ 192kbps)
- **适用场景**:
  - 游戏音频
  - 开源项目
  - Linux 系统
- **文件大小**: 略小于 MP3,音质更好

#### 7. OPUS (Opus Interactive Audio Codec)
- **编码器**: libopus
- **特点**:
  - 最现代的音频编码器
  - 低延迟,高效率
  - 在低比特率下表现优异
- **比特率选项**: 64/96/128/192/256 kbps (默认 128kbps)
- **适用场景**:
  - 实时通信
  - 流媒体
  - 网络传输
  - 低带宽环境
- **文件大小**: 最小,128kbps 下音质接近 MP3 192kbps

## 格式对比

| 格式 | 类型 | 音质 | 文件大小 | 兼容性 | 推荐场景 |
|------|------|------|----------|--------|----------|
| **WAV** | 无损 | ⭐⭐⭐⭐⭐ | 最大 | ⭐⭐⭐⭐⭐ | 专业制作 |
| **FLAC** | 无损 | ⭐⭐⭐⭐⭐ | 大 | ⭐⭐⭐⭐ | 音乐收藏 |
| **MP3** | 有损 | ⭐⭐⭐⭐ | 中 | ⭐⭐⭐⭐⭐ | 日常使用 |
| **AAC** | 有损 | ⭐⭐⭐⭐ | 中 | ⭐⭐⭐⭐ | Apple 设备 |
| **M4A** | 有损 | ⭐⭐⭐⭐ | 中 | ⭐⭐⭐⭐ | iTunes |
| **OGG** | 有损 | ⭐⭐⭐⭐ | 小 | ⭐⭐⭐ | 游戏/开源 |
| **OPUS** | 有损 | ⭐⭐⭐⭐ | 最小 | ⭐⭐⭐ | 流媒体 |

## 质量预设

### 低质量 (128 kbps)
- 适合: 语音、播客、预览
- 文件大小: ~1MB/分钟
- 音质: 可接受

### 中等质量 (192 kbps)
- 适合: 日常听歌、移动设备
- 文件大小: ~1.5MB/分钟
- 音质: 良好

### 高质量 (256 kbps)
- 适合: 高品质音乐、收藏
- 文件大小: ~2MB/分钟
- 音质: 优秀

### 无损 (320 kbps / FLAC / WAV)
- 适合: 专业制作、发烧友
- 文件大小: 2.5-10MB/分钟
- 音质: 完美

## 使用建议

### 按用途选择

**专业音频制作**:
```
格式: WAV 或 FLAC
质量: 无损
采样率: 48000 Hz
```

**音乐收藏**:
```
格式: FLAC 或 MP3
质量: 高 (256 kbps) 或无损
采样率: 44100 Hz
```

**日常听歌**:
```
格式: MP3 或 AAC
质量: 中 (192 kbps) 或高 (256 kbps)
采样率: 44100 Hz
```

**移动设备**:
```
格式: AAC 或 M4A (iOS) / MP3 (Android)
质量: 中 (192 kbps)
采样率: 44100 Hz
```

**网络分享**:
```
格式: MP3 或 OPUS
质量: 中 (192 kbps) 或低 (128 kbps)
采样率: 44100 Hz
```

**游戏开发**:
```
格式: OGG
质量: 中 (192 kbps)
采样率: 44100 Hz
```

## 技术实现

### FFmpeg 编码参数

#### WAV
```bash
ffmpeg -i input.wav -codec:a pcm_s16le output.wav
```

#### FLAC
```bash
ffmpeg -i input.wav -codec:a flac output.flac
```

#### MP3
```bash
ffmpeg -i input.wav -codec:a libmp3lame -b:a 192k output.mp3
```

#### AAC
```bash
ffmpeg -i input.wav -codec:a aac -b:a 192k output.aac
```

#### M4A
```bash
ffmpeg -i input.wav -codec:a aac -b:a 192k output.m4a
```

#### OGG
```bash
# 比特率模式
ffmpeg -i input.wav -codec:a libvorbis -b:a 192k output.ogg

# 质量模式 (推荐)
ffmpeg -i input.wav -codec:a libvorbis -q:a 6 output.ogg
```

#### OPUS
```bash
ffmpeg -i input.wav -codec:a libopus -b:a 128k output.opus
```

### 采样率选项

- **44100 Hz**: CD 音质标准,适合大多数场景
- **48000 Hz**: 专业音频/视频制作标准

## 依赖要求

所有格式转换依赖 **FFmpeg**,需要确保系统已安装:

```bash
# macOS
brew install ffmpeg

# Ubuntu/Debian
sudo apt install ffmpeg

# Windows
# 下载并安装 FFmpeg,添加到 PATH
```

### 编码器支持检查

```bash
# 检查 FFmpeg 支持的编码器
ffmpeg -encoders | grep -E "mp3|aac|flac|vorbis|opus"
```

## 常见问题

### Q: 为什么某些格式转换失败?
A: 确保 FFmpeg 已正确安装并包含所需的编码器。某些系统的 FFmpeg 可能不包含专利编码器(如 AAC)。

### Q: 哪种格式最好?
A: 取决于用途:
- 专业制作: WAV/FLAC
- 日常使用: MP3/AAC
- 空间受限: OPUS/OGG

### Q: 无损格式之间有区别吗?
A: WAV 和 FLAC 都是无损的,音质完全相同。FLAC 文件更小,支持元数据,但 WAV 兼容性更好。

### Q: OPUS 和 OGG 哪个更好?
A: OPUS 更现代,在低比特率下表现更好,但 OGG 兼容性更广。

### Q: M4A 和 AAC 有什么区别?
A: M4A 是 AAC 编码的 MP4 容器格式,音质相同,M4A 支持更多元数据。

## 更新日志

### v1.1.0 (2026-02-03)
- ✅ 新增 M4A 格式支持
- ✅ 新增 OGG Vorbis 格式支持
- ✅ 新增 OPUS 格式支持
- ✅ 优化格式转换逻辑
- ✅ 添加格式选择 UI
- ✅ 完善 i18n 翻译

---

**维护者**: BGM Extractor Team
**最后更新**: 2026-02-03
