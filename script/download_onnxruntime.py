#!/usr/bin/env python3
"""
下载并打包 ONNX Runtime 库到应用中
"""

import os
import sys
import platform
import urllib.request
import zipfile
import tarfile
import shutil
from pathlib import Path

# ONNX Runtime 版本
ORT_VERSION = "1.23.2"

# 下载 URL
DOWNLOAD_URLS = {
    "darwin_arm64": f"https://github.com/microsoft/onnxruntime/releases/download/v{ORT_VERSION}/onnxruntime-osx-arm64-{ORT_VERSION}.tgz",
    "darwin_x86_64": f"https://github.com/microsoft/onnxruntime/releases/download/v{ORT_VERSION}/onnxruntime-osx-x86_64-{ORT_VERSION}.tgz",
    "linux_x86_64": f"https://github.com/microsoft/onnxruntime/releases/download/v{ORT_VERSION}/onnxruntime-linux-x64-{ORT_VERSION}.tgz",
    "windows_x86_64": f"https://github.com/microsoft/onnxruntime/releases/download/v{ORT_VERSION}/onnxruntime-win-x64-{ORT_VERSION}.zip",
}

# 库文件名
LIB_NAMES = {
    "darwin": "libonnxruntime.dylib",
    "linux": "libonnxruntime.so",
    "windows": "onnxruntime.dll",
}


def get_platform_key():
    """获取当前平台的键"""
    system = platform.system().lower()
    machine = platform.machine().lower()

    if machine in ("arm64", "aarch64"):
        machine = "arm64"
    elif machine in ("x86_64", "amd64"):
        machine = "x86_64"

    return f"{system}_{machine}"


def download_file(url: str, dest: Path):
    """下载文件"""
    print(f"下载: {url}")
    urllib.request.urlretrieve(url, dest)
    print(f"已保存到: {dest}")


def extract_archive(archive_path: Path, dest_dir: Path):
    """解压归档文件"""
    print(f"解压: {archive_path}")

    if archive_path.suffix == ".zip":
        with zipfile.ZipFile(archive_path, 'r') as zf:
            zf.extractall(dest_dir)
    elif archive_path.suffix in (".tgz", ".gz"):
        with tarfile.open(archive_path, 'r:gz') as tf:
            tf.extractall(dest_dir)
    else:
        raise ValueError(f"不支持的归档格式: {archive_path.suffix}")

    print(f"已解压到: {dest_dir}")


def main():
    # 获取脚本目录
    script_dir = Path(__file__).parent
    project_root = script_dir.parent
    binaries_dir = project_root / "src-tauri" / "binaries"

    # 创建目录
    binaries_dir.mkdir(parents=True, exist_ok=True)

    # 获取平台
    platform_key = get_platform_key()
    print(f"当前平台: {platform_key}")

    if platform_key not in DOWNLOAD_URLS:
        print(f"错误: 不支持的平台 {platform_key}")
        sys.exit(1)

    url = DOWNLOAD_URLS[platform_key]
    system = platform.system().lower()
    lib_name = LIB_NAMES.get(system, "libonnxruntime.so")

    # 下载
    archive_name = url.split("/")[-1]
    archive_path = binaries_dir / archive_name

    if not archive_path.exists():
        download_file(url, archive_path)
    else:
        print(f"使用已下载的文件: {archive_path}")

    # 解压
    extract_dir = binaries_dir / "onnxruntime_temp"
    if extract_dir.exists():
        shutil.rmtree(extract_dir)
    extract_dir.mkdir()

    extract_archive(archive_path, extract_dir)

    # 查找库文件
    lib_file = None
    for root, dirs, files in os.walk(extract_dir):
        for f in files:
            if f == lib_name or f.startswith(lib_name.replace(".dylib", "").replace(".so", "")):
                lib_file = Path(root) / f
                break
        if lib_file:
            break

    if not lib_file:
        print(f"错误: 找不到库文件 {lib_name}")
        sys.exit(1)

    # 复制库文件
    target_suffix = {
        "darwin_arm64": "aarch64-apple-darwin",
        "darwin_x86_64": "x86_64-apple-darwin",
        "linux_x86_64": "x86_64-unknown-linux-gnu",
        "windows_x86_64": "x86_64-pc-windows-msvc",
    }.get(platform_key, platform_key)

    target_name = f"libonnxruntime-{target_suffix}"
    if system == "darwin":
        target_name += ".dylib"
    elif system == "linux":
        target_name += ".so"
    else:
        target_name += ".dll"

    target_path = binaries_dir / target_name
    shutil.copy2(lib_file, target_path)
    print(f"已复制库文件到: {target_path}")

    # 清理
    shutil.rmtree(extract_dir)
    print("清理完成")

    print(f"\n成功! ONNX Runtime 库已安装到: {target_path}")
    print(f"在 Rust 代码中使用: ort::init_from(\"{target_path}\")?.commit();")


if __name__ == "__main__":
    main()
