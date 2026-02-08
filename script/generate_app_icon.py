#!/usr/bin/env python3
"""
Generate macOS app icon assets from a single source image.

Goal: make the glyph (note + sparkles) smaller inside a full-bleed background,
so Launchpad/Dock doesn't look "too big".

This script:
1) Extracts the foreground by keying out near-black background into alpha.
2) Scales the foreground layer down and centers it on a solid background.
3) Applies rounded-corner alpha mask (macOS-style).

Requires Pillow (PIL).
"""

from __future__ import annotations

import argparse
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw


def build_soft_alpha(img: Image.Image, low: int, high: int) -> Image.Image:
    """Create a soft alpha mask based on brightness to remove near-black background."""
    rgb = img.convert("RGB")
    r, g, b = rgb.split()
    brightness = ImageChops.lighter(ImageChops.lighter(r, g), b)  # max channel

    def ramp(v: int) -> int:
        if v <= low:
            return 0
        if v >= high:
            return 255
        return int((v - low) * 255 / max(1, (high - low)))

    return brightness.point(ramp, mode="L")


def rounded_rect_mask(size: tuple[int, int], radius: int) -> Image.Image:
    w, h = size
    mask = Image.new("L", (w, h), 0)
    draw = ImageDraw.Draw(mask)
    draw.rounded_rectangle((0, 0, w, h), radius=radius, fill=255)
    return mask


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--input", required=True, help="Source PNG (1024x1024 recommended)")
    p.add_argument("--output", required=True, help="Output PNG (same size as input)")
    p.add_argument("--bg", default="#000000", help="Background color (hex), default black")
    p.add_argument(
        "--padding",
        type=int,
        default=None,
        help="Transparent padding in px around the background (overrides --bg-scale)",
    )
    p.add_argument(
        "--bg-scale",
        type=float,
        default=0.84,
        help="Background rounded-rect scale vs canvas (adds transparent padding outside)",
    )
    p.add_argument(
        "--scale",
        type=float,
        default=0.72,
        help="Foreground scale vs background (glyph size inside the rounded rect)",
    )
    p.add_argument("--key-low", type=int, default=6, help="Black key low threshold (0-255)")
    p.add_argument("--key-high", type=int, default=30, help="Black key high threshold (0-255)")
    p.add_argument("--corner-radius", type=float, default=0.19, help="Corner radius as fraction of background size")
    args = p.parse_args()

    src_path = Path(args.input)
    dst_path = Path(args.output)

    img = Image.open(src_path).convert("RGBA")
    w, h = img.size

    alpha_soft = build_soft_alpha(img, low=args.key_low, high=args.key_high)
    r, g, b, a = img.split()
    a2 = ImageChops.multiply(a, alpha_soft)
    fg = Image.merge("RGBA", (r, g, b, a2))

    canvas = Image.new("RGBA", (w, h), (0, 0, 0, 0))

    if args.padding is not None:
        bg_w = max(1, w - 2 * args.padding)
        bg_h = max(1, h - 2 * args.padding)
    else:
        bg_w = max(1, int(w * args.bg_scale))
        bg_h = max(1, int(h * args.bg_scale))
    bg_layer = Image.new("RGBA", (bg_w, bg_h), (0, 0, 0, 0))
    radius = int(min(bg_w, bg_h) * args.corner_radius)
    bg_mask = rounded_rect_mask((bg_w, bg_h), radius)
    bg_fill = Image.new("RGBA", (bg_w, bg_h), args.bg)
    bg_fill.putalpha(bg_mask)

    fg_w = max(1, int(bg_w * args.scale))
    fg_h = max(1, int(bg_h * args.scale))
    fg_scaled = fg.resize((fg_w, fg_h), resample=Image.Resampling.LANCZOS)

    bg_fill.alpha_composite(fg_scaled, ((bg_w - fg_w) // 2, (bg_h - fg_h) // 2))
    canvas.alpha_composite(bg_fill, ((w - bg_w) // 2, (h - bg_h) // 2))

    dst_path.parent.mkdir(parents=True, exist_ok=True)
    canvas.save(dst_path, "PNG")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
