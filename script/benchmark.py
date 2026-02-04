#!/usr/bin/env python3
"""
测试 demucs 处理一首歌需要的时间
"""

import argparse
import subprocess
import sys
import time
from pathlib import Path


def get_audio_duration(file_path: str) -> float:
    """获取音频文件时长（秒）"""
    try:
        result = subprocess.run(
            ["ffprobe", "-v", "error", "-show_entries", "format=duration",
             "-of", "default=noprint_wrappers=1:nokey=1", file_path],
            capture_output=True, text=True, check=True
        )
        return float(result.stdout.strip())
    except Exception:
        return 0


def benchmark(input_file: str, model: str = "htdemucs"):
    """
    测试处理时间

    Args:
        input_file: 输入音频文件路径
        model: 使用的模型
    """
    input_path = Path(input_file)

    if not input_path.exists():
        print(f"错误: 文件不存在 - {input_file}")
        sys.exit(1)

    output_dir = Path("/tmp/demucs_benchmark")
    output_dir.mkdir(parents=True, exist_ok=True)

    # 获取音频时长
    duration = get_audio_duration(input_file)

    print("=" * 50)
    print("Demucs 性能测试")
    print("=" * 50)
    print(f"输入文件: {input_path.name}")
    if duration > 0:
        print(f"音频时长: {duration:.1f} 秒 ({duration/60:.1f} 分钟)")
    print(f"使用模型: {model}")
    print("-" * 50)

    cmd = [
        "demucs",
        "--two-stems=vocals",
        "-n", model,
        "-o", str(output_dir),
        str(input_path)
    ]

    print("开始处理...")
    start_time = time.time()

    try:
        subprocess.run(cmd, check=True)
    except subprocess.CalledProcessError as e:
        print(f"处理失败: {e}")
        sys.exit(1)

    end_time = time.time()
    elapsed = end_time - start_time

    print("-" * 50)
    print("测试结果:")
    print(f"  处理耗时: {elapsed:.1f} 秒 ({elapsed/60:.1f} 分钟)")
    if duration > 0:
        ratio = elapsed / duration
        print(f"  处理速度: {ratio:.2f}x 实时 (1x = 实时处理)")
        print(f"  即: 处理 1 分钟音频需要 {ratio:.1f} 分钟")
    print("=" * 50)


def main():
    parser = argparse.ArgumentParser(description="测试 demucs 处理时间")
    parser.add_argument("input", help="输入音频文件路径")
    parser.add_argument(
        "-m", "--model",
        default="htdemucs",
        choices=["htdemucs", "htdemucs_ft", "mdx_extra"],
        help="分离模型（默认: htdemucs）"
    )

    args = parser.parse_args()
    benchmark(args.input, args.model)


if __name__ == "__main__":
    main()
