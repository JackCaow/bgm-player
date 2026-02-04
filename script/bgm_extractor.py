#!/usr/bin/env python3
"""
BGM Extractor - 独立可执行版本
使用 demucs 进行音频分离，支持进度输出
"""

import argparse
import sys
import os

# 设置环境变量
os.environ["PYTORCH_ENABLE_MPS_FALLBACK"] = "1"

def save_wav(tensor, path, sample_rate):
    """使用 scipy 保存 WAV 文件"""
    import numpy as np
    from scipy.io import wavfile

    # 转换为 numpy 数组
    audio = tensor.numpy()

    # 如果是多声道，转置为 (samples, channels)
    if audio.ndim == 2:
        audio = audio.T

    # 转换为 16-bit PCM
    audio = np.clip(audio, -1.0, 1.0)
    audio = (audio * 32767).astype(np.int16)

    wavfile.write(str(path), sample_rate, audio)

def main():
    parser = argparse.ArgumentParser(description="提取歌曲背景音乐（去除人声）")
    parser.add_argument("input", help="输入音频文件路径")
    parser.add_argument("-o", "--output", help="输出目录")
    parser.add_argument(
        "-m", "--model",
        default="htdemucs",
        choices=["htdemucs", "htdemucs_ft", "htdemucs_6s", "mdx_extra"],
        help="分离模型（默认: htdemucs）"
    )
    parser.add_argument(
        "-d", "--device",
        choices=["cpu", "mps", "cuda"],
        help="计算设备（默认: 自动选择）"
    )
    parser.add_argument(
        "--two-stems",
        default=None,
        help="只分离指定的两个轨道（如 vocals），不指定则输出所有轨道"
    )

    args = parser.parse_args()

    # 延迟导入
    import torch
    from pathlib import Path

    input_path = Path(args.input)
    if not input_path.exists():
        print(f"错误: 文件不存在 - {args.input}", file=sys.stderr)
        sys.exit(1)

    # 自动选择设备
    device = args.device
    if device is None:
        if torch.backends.mps.is_available():
            device = "mps"
        elif torch.cuda.is_available():
            device = "cuda"
        else:
            device = "cpu"

    # 设置输出目录
    if args.output:
        output_dir = Path(args.output)
    else:
        output_dir = Path.cwd() / "output"

    output_dir.mkdir(parents=True, exist_ok=True)

    print(f"输入文件: {input_path.name}", file=sys.stderr)
    print(f"使用模型: {args.model}", file=sys.stderr)
    print(f"计算设备: {device}", file=sys.stderr)
    print(f"输出目录: {output_dir}", file=sys.stderr)

    # 导入 demucs
    from demucs.pretrained import get_model
    from demucs.apply import apply_model
    from demucs.audio import AudioFile

    print("正在加载模型...", file=sys.stderr)
    model = get_model(args.model)
    model.to(device)
    model.eval()

    print("正在读取音频...", file=sys.stderr)
    wav = AudioFile(input_path).read(
        streams=0,
        samplerate=model.samplerate,
        channels=model.audio_channels
    )
    ref = wav.mean(0)
    wav = (wav - ref.mean()) / ref.std()

    print("正在分离音轨...", file=sys.stderr)

    # 使用 apply_model 进行分离
    sources = apply_model(
        model,
        wav[None].to(device),
        device=device,
        progress=True,
        num_workers=0
    )[0]

    sources = sources * ref.std() + ref.mean()

    # 保存结果
    stem_name = input_path.stem
    result_dir = output_dir / args.model / stem_name
    result_dir.mkdir(parents=True, exist_ok=True)

    print("正在保存文件...", file=sys.stderr)

    # 获取源名称
    source_names = model.sources

    if args.two_stems:
        stem = args.two_stems
        if stem in source_names:
            idx = source_names.index(stem)
            # 保存指定轨道
            stem_path = result_dir / f"{stem}.wav"
            save_wav(sources[idx].cpu(), stem_path, model.samplerate)

            # 保存其余轨道的混合
            other = sources.sum(0) - sources[idx]
            other_path = result_dir / f"no_{stem}.wav"
            save_wav(other.cpu(), other_path, model.samplerate)

            print(f"已保存: {stem_path}", file=sys.stderr)
            print(f"已保存: {other_path}", file=sys.stderr)
    else:
        for idx, name in enumerate(source_names):
            stem_path = result_dir / f"{name}.wav"
            save_wav(sources[idx].cpu(), stem_path, model.samplerate)
            print(f"已保存: {stem_path}", file=sys.stderr)

    print("处理完成!", file=sys.stderr)

if __name__ == "__main__":
    main()
