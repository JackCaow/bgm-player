#!/usr/bin/env python3
"""
打包 BGM Extractor 为独立可执行文件
支持 macOS (arm64/x86_64), Windows, Linux
"""

import subprocess
import sys
import os
from pathlib import Path

def main():
    script_dir = Path(__file__).parent
    project_root = script_dir.parent

    # 目标脚本
    target_script = script_dir / "bgm_extractor.py"

    # 输出目录
    dist_dir = project_root / "src-tauri" / "binaries"
    dist_dir.mkdir(parents=True, exist_ok=True)

    # 获取平台标识
    import platform
    system = platform.system().lower()
    machine = platform.machine().lower()

    if system == "darwin":
        if machine == "arm64":
            target_triple = "aarch64-apple-darwin"
        else:
            target_triple = "x86_64-apple-darwin"
    elif system == "linux":
        if machine == "aarch64":
            target_triple = "aarch64-unknown-linux-gnu"
        else:
            target_triple = "x86_64-unknown-linux-gnu"
    elif system == "windows":
        target_triple = "x86_64-pc-windows-msvc"
    else:
        target_triple = f"{machine}-{system}"

    output_name = f"bgm-extractor-{target_triple}"

    print(f"=" * 50)
    print(f"BGM Extractor 打包工具")
    print(f"=" * 50)
    print(f"目标平台: {target_triple}")
    print(f"输出名称: {output_name}")
    print(f"输出目录: {dist_dir}")
    print("-" * 50)

    # PyInstaller 参数
    cmd = [
        sys.executable, "-m", "PyInstaller",
        "--onefile",
        "--name", output_name,
        "--distpath", str(dist_dir),
        "--workpath", str(project_root / "build" / "pyinstaller"),
        "--specpath", str(project_root / "build"),
        # 收集 demucs 相关数据
        "--collect-data", "demucs",
        "--collect-submodules", "demucs",
        # 收集 torch 相关
        "--collect-submodules", "torch",
        "--collect-data", "torch",
        # 隐藏导入
        "--hidden-import", "demucs",
        "--hidden-import", "demucs.pretrained",
        "--hidden-import", "demucs.apply",
        "--hidden-import", "demucs.audio",
        "--hidden-import", "torch",
        "--hidden-import", "torchaudio",
        "--hidden-import", "numpy",
        "--hidden-import", "scipy",
        "--hidden-import", "scipy.io",
        "--hidden-import", "scipy.io.wavfile",
        # 排除不需要的模块（保守策略，只排除非 torch 核心模块）
        "--exclude-module", "torchcodec",
        "--exclude-module", "tensorboard",
        "--exclude-module", "matplotlib",
        "--exclude-module", "caffe2",
        "--exclude-module", "IPython",
        "--exclude-module", "jupyter",
        "--exclude-module", "notebook",
        "--exclude-module", "PIL",
        "--exclude-module", "cv2",
        "--exclude-module", "sklearn",
        "--exclude-module", "pandas",
        # 清理
        "--clean",
        "--noconfirm",
        # 目标脚本
        str(target_script),
    ]

    print("开始打包...")
    print("-" * 50)

    result = subprocess.run(cmd, cwd=project_root)

    if result.returncode == 0:
        output_path = dist_dir / output_name
        if system == "windows":
            output_path = output_path.with_suffix(".exe")

        print("-" * 50)
        print(f"打包成功!")
        print(f"输出文件: {output_path}")

        if output_path.exists():
            size_mb = output_path.stat().st_size / 1024 / 1024
            print(f"文件大小: {size_mb:.1f} MB")

        # 提示其他平台打包
        print("-" * 50)
        print("提示: 要打包其他平台版本，请在对应平台上运行此脚本")
        print("  - macOS (Apple Silicon): aarch64-apple-darwin")
        print("  - macOS (Intel): x86_64-apple-darwin")
        print("  - Windows: x86_64-pc-windows-msvc")
        print("  - Linux: x86_64-unknown-linux-gnu")
    else:
        print("打包失败!")
        sys.exit(1)

if __name__ == "__main__":
    main()
