#!/usr/bin/env python3
"""
提取歌曲背景音乐（去除人声）
使用 demucs 进行音频分离
"""

import argparse
import subprocess
import sys
from pathlib import Path


def check_demucs():
    """检查 demucs 是否已安装"""
    try:
        subprocess.run(
            ["demucs", "--help"],
            capture_output=True,
            check=True
        )
        return True
    except (subprocess.CalledProcessError, FileNotFoundError):
        return False


def install_demucs():
    """安装 demucs"""
    print("正在安装 demucs...")
    subprocess.run(
        [sys.executable, "-m", "pip", "install", "demucs"],
        check=True
    )


def get_best_device():
    """获取最佳计算设备"""
    try:
        import torch
        if torch.backends.mps.is_available():
            return "mps"
        elif torch.cuda.is_available():
            return "cuda"
    except ImportError:
        pass
    return "cpu"


def extract_bgm(input_file: str, output_dir: str = None, model: str = "htdemucs", device: str = None):
    """
    提取背景音乐

    Args:
        input_file: 输入音频文件路径
        output_dir: 输出目录，默认为当前目录下的 output 文件夹
        model: 使用的模型，默认 htdemucs
        device: 计算设备 (cpu/mps/cuda)，默认自动选择
    """
    input_path = Path(input_file)

    if not input_path.exists():
        print(f"错误: 文件不存在 - {input_file}")
        sys.exit(1)

    if output_dir is None:
        output_dir = Path.cwd() / "output"
    else:
        output_dir = Path(output_dir)

    if device is None:
        device = get_best_device()

    output_dir.mkdir(parents=True, exist_ok=True)

    print(f"正在处理: {input_path.name}")
    print(f"输出目录: {output_dir}")
    print(f"使用模型: {model}")
    print(f"计算设备: {device}")
    print("-" * 40)

    # 使用 demucs 分离音频
    # --two-stems=vocals 只分离人声和伴奏两个轨道
    cmd = [
        "demucs",
        "--two-stems=vocals",
        "-n", model,
        "-d", device,
        "-o", str(output_dir),
        str(input_path)
    ]

    try:
        subprocess.run(cmd, check=True)
    except subprocess.CalledProcessError as e:
        print(f"处理失败: {e}")
        sys.exit(1)

    # 输出结果路径
    stem_name = input_path.stem
    result_dir = output_dir / model / stem_name
    bgm_file = result_dir / "no_vocals.wav"
    vocals_file = result_dir / "vocals.wav"

    print("-" * 40)
    print("处理完成!")
    print(f"背景音乐: {bgm_file}")
    print(f"人声轨道: {vocals_file}")

    return bgm_file


def main():
    parser = argparse.ArgumentParser(
        description="提取歌曲背景音乐（去除人声）"
    )
    parser.add_argument(
        "input",
        help="输入音频文件路径"
    )
    parser.add_argument(
        "-o", "--output",
        help="输出目录（默认: ./output）"
    )
    parser.add_argument(
        "-m", "--model",
        default="htdemucs",
        choices=["htdemucs", "htdemucs_ft", "mdx_extra"],
        help="分离模型（默认: htdemucs）"
    )
    parser.add_argument(
        "-d", "--device",
        choices=["cpu", "mps", "cuda"],
        help="计算设备（默认: 自动选择最佳设备）"
    )

    args = parser.parse_args()

    # 检查并安装 demucs
    if not check_demucs():
        install_demucs()

    extract_bgm(args.input, args.output, args.model, args.device)


if __name__ == "__main__":
    main()
