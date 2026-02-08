#!/usr/bin/env python3
"""CosyVoice3 TTS script with zero-shot and instruct support.

Usage examples:
  # Zero-shot voice cloning (requires reference audio)
  python3 cosyvoice_tts.py --text "Hello world" --mode zero_shot --ref-audio ref.wav --model-dir tts_models/cosyvoice3 --output out.wav

  # Instruct mode (dialect/speed control)
  python3 cosyvoice_tts.py --text "你好世界" --mode instruct --instruct "请用广东话表达。" --ref-audio ref.wav --model-dir tts_models/cosyvoice3 --output out.wav

  # Cross-lingual synthesis
  python3 cosyvoice_tts.py --text "Hello world" --mode cross_lingual --ref-audio ref.wav --model-dir tts_models/cosyvoice3 --output out.wav

  # SFT mode (legacy, if model supports)
  python3 cosyvoice_tts.py --text "你好" --mode sft --voice default --model-dir tts_models/cosyvoice3 --output out.wav
"""

from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path

def _safe_stderr(msg: str) -> None:
    try:
        print(msg, file=sys.stderr)
    except BrokenPipeError:
        pass


def _score_cosyvoice_model_dir(path: Path) -> int:
    score = 0
    markers = [
        "llm.pt",
        "flow.pt",
        "hift.pt",
        "speech_tokenizer_v3.onnx",
        "campplus.onnx",
    ]
    for name in markers:
        if (path / name).exists():
            score += 1
    return score


def _resolve_cosyvoice_model_dir(input_dir: Path) -> Path:
    """Resolve model dir when user passes a parent directory."""
    if not input_dir.exists() or not input_dir.is_dir():
        return input_dir

    best = input_dir
    best_score = _score_cosyvoice_model_dir(input_dir)
    if best_score >= 2:
        return best

    # Search 1~2 levels for the real model root (e.g. Fun-CosyVoice3-0.5B-2512)
    for child in input_dir.iterdir():
        if child.is_dir():
            score = _score_cosyvoice_model_dir(child)
            if score > best_score:
                best = child
                best_score = score
            for sub in child.iterdir():
                if sub.is_dir():
                    score2 = _score_cosyvoice_model_dir(sub)
                    if score2 > best_score:
                        best = sub
                        best_score = score2
    return best

# Add CosyVoice to path if available
def _setup_cosyvoice_path():
    """Setup Python path for CosyVoice imports."""
    script_dir = Path(__file__).parent.resolve()
    project_root = script_dir.parent

    # Search for CosyVoice in multiple locations
    candidates = [
        project_root / "third_party" / "CosyVoice",
        script_dir.parent / "third_party" / "CosyVoice",
        project_root.parent / "third_party" / "CosyVoice",
        project_root.parent.parent / "third_party" / "CosyVoice",
        Path.cwd() / "third_party" / "CosyVoice",
        Path.cwd().parent / "third_party" / "CosyVoice",
        Path.home() / "CosyVoice",
        Path("/opt/CosyVoice"),
    ]

    # Also check COSYVOICE_PATH environment variable
    if os.environ.get("COSYVOICE_PATH"):
        candidates.insert(0, Path(os.environ["COSYVOICE_PATH"]))

    for cosyvoice_dir in candidates:
        if cosyvoice_dir.exists() and (cosyvoice_dir / "cosyvoice").exists():
            sys.path.insert(0, str(cosyvoice_dir))
            matcha_dir = cosyvoice_dir / "third_party" / "Matcha-TTS"
            if matcha_dir.exists():
                sys.path.insert(0, str(matcha_dir))
            return cosyvoice_dir
    _safe_stderr("CosyVoice source directory not found in known locations.")
    return None

_cosyvoice_dir = _setup_cosyvoice_path()

import numpy as np


def _get_default_ref_audio(model_dir: Path | None = None) -> Path | None:
    """Get default reference audio from CosyVoice assets."""
    script_dir = Path(__file__).parent.resolve()
    project_root = script_dir.parent

    candidates = [
        model_dir / "asset" / "zero_shot_prompt.wav" if model_dir else None,
        # From CosyVoice directory
        _cosyvoice_dir / "asset" / "zero_shot_prompt.wav" if _cosyvoice_dir else None,
        # From project root
        project_root / "third_party" / "CosyVoice" / "asset" / "zero_shot_prompt.wav",
    ]
    for p in candidates:
        if p and p.exists():
            return p
    return None


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Synthesize speech with CosyVoice3")
    parser.add_argument("--text", required=True, help="Input text to synthesize")
    parser.add_argument(
        "--mode",
        choices=["zero_shot", "instruct", "cross_lingual", "sft"],
        default="zero_shot",
        help="Inference mode (default: zero_shot)",
    )
    parser.add_argument("--voice", default="default", help="Speaker id for SFT mode")
    parser.add_argument("--ref-audio", help="Reference audio for zero-shot/instruct/cross-lingual modes")
    parser.add_argument("--ref-text", help="Reference text (prompt) for zero-shot mode")
    parser.add_argument("--instruct", default="", help="Instruct prompt for instruct mode (e.g., '请用广东话表达。')")
    parser.add_argument("--language", default="zh-cn", help="Language hint")
    parser.add_argument("--speed", type=float, default=1.0, help="Speech speed (0.6-1.6)")
    parser.add_argument("--stream", action="store_true", help="Enable streaming output")
    parser.add_argument("--model-dir", required=True, help="CosyVoice model directory")
    parser.add_argument("--output", required=True, help="Output WAV path")
    return parser.parse_args()


def _import_cosyvoice():
    """Import CosyVoice model class (AutoModel for v3, CosyVoice for legacy)."""
    # Try AutoModel first (CosyVoice3)
    try:
        from cosyvoice.cli.cosyvoice import AutoModel
        return AutoModel, "auto"
    except ImportError:
        pass
    # Fallback to legacy CosyVoice
    try:
        from cosyvoice.cli.cosyvoice import CosyVoice
        return CosyVoice, "legacy"
    except ImportError:
        pass
    try:
        from cli.cosyvoice import CosyVoice
        return CosyVoice, "legacy"
    except ImportError as exc:
        raise ImportError("Cannot import CosyVoice. Please install CosyVoice dependencies.") from exc


def _to_numpy_audio(item) -> np.ndarray:
    audio = item.get("tts_speech")
    if audio is None:
        raise RuntimeError("CosyVoice output missing tts_speech")
    if hasattr(audio, "detach"):
        audio = audio.detach().cpu().numpy()
    audio = np.asarray(audio)
    if audio.ndim == 0:
        return np.zeros((0,), dtype=np.float32)
    if audio.ndim == 1:
        return audio.astype(np.float32)
    return audio.reshape(-1).astype(np.float32)


def _get_sample_rate(chunks, default: int = 22050) -> int:
    for c in reversed(chunks):
        if "sample_rate" in c:
            return int(c["sample_rate"])
    return default


def _build_ref_prompt(ref_text: str | None, language: str) -> str:
    """Build a short reference prompt for zero-shot inference."""
    if ref_text:
        return ref_text.strip()
    lang = (language or "").lower()
    if lang.startswith("zh"):
        seed = "希望你以后能够做的比我还好呦。"
    elif lang.startswith("ja"):
        seed = "こんにちは。"
    else:
        seed = "Hello."
    return f"You are a helpful assistant.<|endofprompt|>{seed}"


def _build_instruct_prompt(instruct: str) -> str:
    """Build instruct prompt for instruct mode."""
    if not instruct:
        return "You are a helpful assistant.<|endofprompt|>"
    return f"You are a helpful assistant. {instruct}<|endofprompt|>"


def main() -> int:
    args = parse_args()
    text = args.text.strip()
    if not text:
        print("Input text is empty", file=sys.stderr)
        return 1

    model_dir = _resolve_cosyvoice_model_dir(
        Path(args.model_dir).expanduser().resolve()
    )
    if not model_dir.exists() or not model_dir.is_dir():
        print(f"Model dir not found: {model_dir}", file=sys.stderr)
        return 1

    output = Path(args.output).expanduser().resolve()
    output.parent.mkdir(parents=True, exist_ok=True)

    speed = max(0.6, min(1.6, float(args.speed)))
    mode = args.mode
    stream = args.stream

    # Validate ref-audio for modes that require it
    ref_audio_path = None
    if mode in ("zero_shot", "instruct", "cross_lingual"):
        if args.ref_audio:
            ref_audio_path = Path(args.ref_audio).expanduser().resolve()
        else:
            # Try to use default reference audio
            ref_audio_path = _get_default_ref_audio(model_dir)
        if not ref_audio_path or not ref_audio_path.exists():
            print(f"--ref-audio is required for {mode} mode (no default found)", file=sys.stderr)
            return 1

    try:
        import torch
        ModelClass, model_type = _import_cosyvoice()
        # Disable fp16 for CPU compatibility on macOS
        cosy = ModelClass(model_dir=str(model_dir), fp16=False)
        # Convert bfloat16 weights to float32 for macOS CPU compatibility
        if not torch.cuda.is_available():
            for name, param in cosy.model.llm.llm.named_parameters():
                if param.dtype == torch.bfloat16:
                    param.data = param.data.to(torch.float32)
    except Exception as exc:
        print(f"Failed to init CosyVoice: {exc}", file=sys.stderr)
        return 1

    sample_rate = getattr(cosy, "sample_rate", 22050)

    try:
        if mode == "zero_shot":
            ref_prompt = _build_ref_prompt(args.ref_text, args.language)
            result = cosy.inference_zero_shot(
                text, ref_prompt, str(ref_audio_path), stream=stream
            )
        elif mode == "instruct":
            instruct_prompt = _build_instruct_prompt(args.instruct)
            result = cosy.inference_instruct2(
                text, instruct_prompt, str(ref_audio_path), stream=stream
            )
        elif mode == "cross_lingual":
            ref_prompt = _build_ref_prompt(args.ref_text, args.language)
            result = cosy.inference_cross_lingual(
                ref_prompt + text, str(ref_audio_path), stream=stream
            )
        elif mode == "sft":
            speaker = args.voice.strip() or "default"
            if hasattr(cosy, "list_available_spks"):
                spks = cosy.list_available_spks() or []
                if spks and speaker not in spks:
                    speaker = spks[0]
            result = cosy.inference_sft(text, speaker, speed=speed, stream=stream)
        else:
            print(f"Unknown mode: {mode}", file=sys.stderr)
            return 1

        if result is None:
            raise RuntimeError("CosyVoice inference returned empty result")

        chunks = list(result) if not isinstance(result, list) else result
        if not chunks:
            raise RuntimeError("CosyVoice inference returned no audio chunks")

        audios = [_to_numpy_audio(c) for c in chunks]
        audio = np.concatenate([a for a in audios if a.size > 0], axis=0)
        if audio.size == 0:
            raise RuntimeError("CosyVoice produced empty audio")

        sample_rate = _get_sample_rate(chunks, sample_rate)

    except Exception as exc:
        print(f"CosyVoice synthesis failed: {exc}", file=sys.stderr)
        return 1

    try:
        import soundfile as sf
        sf.write(str(output), audio, sample_rate)
    except Exception as exc:
        print(f"Failed to write output wav: {exc}", file=sys.stderr)
        return 1

    if not output.exists():
        print("Output file not generated", file=sys.stderr)
        return 1

    print(f"Generated: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
