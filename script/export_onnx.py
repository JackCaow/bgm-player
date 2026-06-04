#!/usr/bin/env python3
"""Export htdemucs to a self-contained ONNX (STFT/iSTFT baked in) via demucs PR #10.

Run inside the project venv:  .venv/bin/python script/export_onnx.py

Background
----------
demucs PR #10 (https://github.com/adefossez/demucs/pull/10) rewrites the STFT/iSTFT
layers as real convolutions so the whole htdemucs model exports to a single ONNX graph
(no external STFT step needed at inference time). The PR lives on the contributor's
fork branch ``dhunstack/demucs@allchanges``; it is unmerged, so we pin to the exact
commit verified for this task.

Notes on the real converter interface (verified by inspecting the cloned repo, NOT the
skeleton's guessed flags):
  * Script:  scripts/convert-pth-to-onnx.py
  * Usage:   python scripts/convert-pth-to-onnx.py <dest_dir>
             -> writes <dest_dir>/htdemucs.onnx
    The model name ("htdemucs") is hardcoded; there is no --model / --out flag.

Verified IO of the produced graph (pretrained htdemucs has segment=39/5s, NOT the 10s
constructor default, so the baked-in segment length T = int(39/5 * 44100) = 343980):
  * input  name = "input"   shape [1, 2, 343980]   float32   (T = 343980 samples ~7.8s)
  * output name = "output"  shape [1, 4, 2, 343980] float32  (= [batch, sources, ch, T])
  * source order = ["drums", "bass", "other", "vocals"]  (vocals = index 3)
  * opset 17. Output dims are fixed (static), NOT dynamic axes -> callers must chunk
    audio into 343980-sample segments and overlap-add (see downstream Rust tasks).

Environment notes:
  * The project venv is uv-managed and has NO pip module, so we install via `uv`.
  * The PR's requirements pin torchaudio<2.2 / torch>=1.8; installing WITH deps would
    downgrade the venv's torch 2.12 / torchaudio 2.11 and break it. We install editable
    with --no-deps (all runtime deps the converter needs are already in the venv).
"""
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BUILD = ROOT / "build" / "demucs-onnx"
OUT = ROOT / "src-tauri" / "resources" / "models" / "htdemucs.onnx"

# demucs PR #10 comes from a contributor fork branch (not pull/<N>/head on the upstream).
PR_REMOTE = "https://github.com/dhunstack/demucs.git"
PR_BRANCH = "allchanges"
# Pin to the exact verified commit (PR #10 head as of this task).
PR_COMMIT = "6a96f8c0c64474d0b5a91f5cc680e3f33b0ed185"

# uv binary: prefer PATH, fall back to the standard user install location.
UV = shutil.which("uv") or str(Path.home() / ".local" / "bin" / "uv")


def run(cmd, **kw):
    print("+", " ".join(map(str, cmd)))
    subprocess.run(cmd, check=True, **kw)


def _export_htdemucs(build_dir: Path) -> Path:
    """Mirror scripts/convert-pth-to-onnx.py from PR #10, forcing the legacy exporter.

    Returns the path to the produced .onnx (under build/onnx-out/).
    """
    import torch
    from torch.nn import functional as F
    from demucs.pretrained import get_model
    from demucs.htdemucs import HTDemucs

    model_name = "htdemucs"
    model = get_model(model_name)  # downloads pretrained weights on first run
    if isinstance(model, HTDemucs):
        core = model
    elif hasattr(model, "models") and isinstance(model.models[0], HTDemucs):
        core = model.models[0]  # unwrap BagOfModels
    else:
        raise TypeError(f"Unsupported model type: {type(model)!r}")

    core.onnx_exportable = True

    # Same dummy as the PR: 343980 samples, padded up to training_length if needed.
    dummy = torch.randn(1, 2, 343980)
    training_length = int(core.segment * core.samplerate)
    if dummy.shape[-1] < training_length:
        dummy = F.pad(dummy, (0, training_length - dummy.shape[-1]))

    dest_dir = ROOT / "build" / "onnx-out"
    dest_dir.mkdir(parents=True, exist_ok=True)
    onnx_path = dest_dir / f"{model_name}.onnx"

    print(f"Converting {model_name} to ONNX (legacy exporter, dynamo=False)...")
    print(f"  segment={core.segment} samplerate={core.samplerate} "
          f"dummy_len={dummy.shape[-1]}")
    torch.onnx.export(
        core,
        (dummy,),
        str(onnx_path),
        export_params=True,
        opset_version=17,
        do_constant_folding=True,
        input_names=["input"],
        output_names=["output"],
        dynamo=False,
    )
    if not onnx_path.exists():
        raise SystemExit(f"torch.onnx.export did not produce {onnx_path}")
    return onnx_path


def main():
    OUT.parent.mkdir(parents=True, exist_ok=True)

    # 1. Clone + pin the PR branch.
    if not BUILD.exists():
        BUILD.parent.mkdir(parents=True, exist_ok=True)
        run(["git", "clone", PR_REMOTE, str(BUILD)])
        run(["git", "-C", str(BUILD), "fetch", "origin", PR_BRANCH])
    run(["git", "-C", str(BUILD), "checkout", PR_COMMIT])

    # 2. Install editable into the existing venv WITHOUT touching its deps (protects
    #    torch/torchaudio). The venv has no pip, so use uv.
    run([UV, "pip", "install", "--python", sys.executable, "-e", str(BUILD), "--no-deps"])

    # 3. Export. This reproduces the PR's converter setup
    #    (scripts/convert-pth-to-onnx.py: load htdemucs, flip onnx_exportable, pad a
    #    dummy waveform to the training length, torch.onnx.export with opset 17, input
    #    name "input" / output name "output"), with ONE necessary adaptation:
    #
    #    The PR predates torch 2.x's dynamo ONNX exporter. On this venv's torch 2.12,
    #    torch.onnx.export defaults to dynamo=True, which fails on a data-dependent
    #    assertion in demucs' pad1d (hdemucs.py:39) -> GuardOnDataDependentSymNode. We
    #    force dynamo=False to use the legacy TorchScript tracer the PR was written for;
    #    that path traces cleanly. We do this in-process (rather than shelling out to the
    #    PR's script) only so we can pass dynamo=False without patching the pinned PR.
    produced = _export_htdemucs(BUILD)

    # 4. Copy into the Tauri resources tree.
    shutil.copy2(produced, OUT)
    print(f"Exported -> {OUT} ({OUT.stat().st_size / 1e6:.1f} MB)")

    # 5. Inspect the produced graph (best effort; onnx may be missing).
    try:
        import onnx  # noqa: WPS433

        model = onnx.load(str(OUT))
        g = model.graph

        def shape_of(value_info):
            dims = []
            for d in value_info.type.tensor_type.shape.dim:
                dims.append(d.dim_value if d.HasField("dim_value") else (d.dim_param or "?"))
            return dims

        print("opset:", [(o.domain or "ai.onnx", o.version) for o in model.opset_import])
        for inp in g.input:
            print("IN ", inp.name, shape_of(inp))
        for outp in g.output:
            print("OUT", outp.name, shape_of(outp))
    except ImportError:
        print("(onnx not installed; skipping graph inspection -- file still produced)")


if __name__ == "__main__":
    main()
