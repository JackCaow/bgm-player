#!/usr/bin/env python3
"""根据源图生成符合 macOS 规范的 app 图标。

macOS 的 app 图标不会被系统自动裁切圆角(不同于 iOS),图片本身必须是
"圆角矩形 + 四周透明留白 + 轻微阴影"的悬浮块。本脚本把源图放进居中的圆角
矩形(1024 画布中约 824 的内容区,四周各留 100px),并生成:

  icon.png / 32x32.png / 128x128.png / 128x128@2x.png / icon.ico
  以及 icon.iconset/ 目录(供外部 iconutil 生成 icon.icns)

用法:
  python3 script/make_macos_icon.py [source.png]
默认源图为 src-tauri/icons/icon.png。
"""
import os
import sys
from PIL import Image, ImageDraw, ImageFilter

HERE = os.path.dirname(os.path.abspath(__file__))
ICONS_DIR = os.path.abspath(os.path.join(HERE, "..", "src-tauri", "icons"))

CANVAS = 1024
MARGIN = 100                       # 四周透明留白
BODY = CANVAS - 2 * MARGIN         # 内容圆角矩形边长 = 824
RADIUS = round(BODY * 0.2237)      # macOS 圆角半径近似 ≈ 184
SHADOW_OFFSET = 10
SHADOW_BLUR = 18
SHADOW_ALPHA = 90


def rounded_mask(size, radius, ss=4):
    """超采样绘制圆角矩形 alpha 遮罩,获得平滑边缘。"""
    big = Image.new("L", (size * ss, size * ss), 0)
    ImageDraw.Draw(big).rounded_rectangle(
        [0, 0, size * ss - 1, size * ss - 1], radius=radius * ss, fill=255
    )
    return big.resize((size, size), Image.LANCZOS)


def build_master(src_path):
    art = Image.open(src_path).convert("RGBA").resize((BODY, BODY), Image.LANCZOS)
    art.putalpha(rounded_mask(BODY, RADIUS))  # 圆角

    canvas = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))

    # 轻微阴影
    shadow = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
    ImageDraw.Draw(shadow).rounded_rectangle(
        [MARGIN, MARGIN + SHADOW_OFFSET, MARGIN + BODY, MARGIN + BODY + SHADOW_OFFSET],
        radius=RADIUS, fill=(0, 0, 0, SHADOW_ALPHA),
    )
    canvas = Image.alpha_composite(canvas, shadow.filter(ImageFilter.GaussianBlur(SHADOW_BLUR)))

    canvas.paste(art, (MARGIN, MARGIN), art)
    return canvas


def main():
    src = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ICONS_DIR, "icon.png")
    master = build_master(src)

    master.save(os.path.join(ICONS_DIR, "icon.png"))
    for name, size in [("32x32.png", 32), ("128x128.png", 128), ("128x128@2x.png", 256)]:
        master.resize((size, size), Image.LANCZOS).save(os.path.join(ICONS_DIR, name))

    master.save(
        os.path.join(ICONS_DIR, "icon.ico"),
        sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )

    iconset = os.path.join(ICONS_DIR, "icon.iconset")
    os.makedirs(iconset, exist_ok=True)
    for name, size in [
        ("icon_16x16.png", 16), ("icon_16x16@2x.png", 32),
        ("icon_32x32.png", 32), ("icon_32x32@2x.png", 64),
        ("icon_128x128.png", 128), ("icon_128x128@2x.png", 256),
        ("icon_256x256.png", 256), ("icon_256x256@2x.png", 512),
        ("icon_512x512.png", 512), ("icon_512x512@2x.png", 1024),
    ]:
        master.resize((size, size), Image.LANCZOS).save(os.path.join(iconset, name))

    print(f"OK  body={BODY} radius={RADIUS} margin={MARGIN}")
    print(f"iconset -> {iconset}")


if __name__ == "__main__":
    main()
