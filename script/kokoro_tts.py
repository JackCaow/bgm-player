#!/usr/bin/env python3
"""Kokoro ONNX TTS helper script.

Requires:
  pip install kokoro-onnx soundfile numpy
"""

from __future__ import annotations

import argparse
import os
import sys


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Synthesize speech with Kokoro ONNX")
    parser.add_argument("--model", required=True, help="Path to Kokoro ONNX model")
    parser.add_argument("--voices", required=True, help="Path to Kokoro voices file")
    parser.add_argument("--text", required=True, help="Text to synthesize")
    parser.add_argument("--voice", default="af_heart", help="Voice id")
    parser.add_argument("--lang", default="en-us", help="Language code")
    parser.add_argument("--speed", type=float, default=1.0, help="Speech speed")
    parser.add_argument("--output", required=True, help="Output wav path")
    return parser.parse_args()


def main() -> int:
    args = parse_args()

    try:
        import soundfile as sf
        from kokoro_onnx import Kokoro
    except Exception as exc:  # pragma: no cover
        print(
            "Missing dependencies. Please install: pip install kokoro-onnx soundfile numpy\n"
            f"Original error: {exc}",
            file=sys.stderr,
        )
        return 2

    try:
        tts = Kokoro(str(args.model), str(args.voices))
        samples, sample_rate = tts.create(
            args.text,
            voice=args.voice,
            speed=float(args.speed),
            lang=args.lang,
        )

        out_dir = os.path.dirname(args.output)
        if out_dir:
            os.makedirs(out_dir, exist_ok=True)

        sf.write(args.output, samples, sample_rate)
        print(args.output)
        return 0
    except Exception as exc:  # pragma: no cover
        print(f"Kokoro synthesis error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
