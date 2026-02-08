"""PyInstaller runtime hook: patch inspect.getsource for TorchScript compatibility.

In --onefile mode, PyInstaller compiles .py to .pyc and the originals are lost.
TorchScript uses inspect.getsource() which fails. This hook patches it to
fall back to reading .py files from the data directory if available.
"""
import inspect
import sys
import os

_original_getsource = inspect.getsource
_original_getsourcelines = inspect.getsourcelines


def _patched_getsource(obj):
    try:
        return _original_getsource(obj)
    except OSError:
        # Try to find the .py file in the PyInstaller data directory
        if hasattr(sys, '_MEIPASS'):
            try:
                source_file = inspect.getfile(obj)
                if source_file.endswith('.pyc'):
                    source_file = source_file[:-1]  # .pyc -> .py
                # Try relative to _MEIPASS
                base = os.path.basename(source_file)
                for root, dirs, files in os.walk(sys._MEIPASS):
                    if base in files:
                        with open(os.path.join(root, base)) as f:
                            return f.read()
            except Exception:
                pass
        raise


def _patched_getsourcelines(obj):
    try:
        return _original_getsourcelines(obj)
    except OSError:
        if hasattr(sys, '_MEIPASS'):
            try:
                source_file = inspect.getfile(obj)
                if source_file.endswith('.pyc'):
                    source_file = source_file[:-1]
                base = os.path.basename(source_file)
                for root, dirs, files in os.walk(sys._MEIPASS):
                    if base in files:
                        with open(os.path.join(root, base)) as f:
                            lines = f.readlines()
                        return lines, 1
            except Exception:
                pass
        raise


inspect.getsource = _patched_getsource
inspect.getsourcelines = _patched_getsourcelines


# Patch torchaudio.load to use soundfile backend instead of torchcodec.
# torchaudio >= 2.9 forces torchcodec which requires native FFmpeg libs
# that are hard to bundle with PyInstaller.
def _patch_torchaudio_load():
    try:
        import torchaudio
        import torch
        import soundfile as sf
        import numpy as np

        _original_load = torchaudio.load

        def _soundfile_load(uri, frame_offset=0, num_frames=-1,
                            normalize=True, channels_first=True,
                            format=None, buffer_size=4096, backend=None):
            data, sample_rate = sf.read(
                uri,
                start=frame_offset,
                stop=frame_offset + num_frames if num_frames > 0 else None,
                dtype='float32',
                always_2d=True,
            )
            waveform = torch.from_numpy(data.T if channels_first else data)
            return waveform, sample_rate

        torchaudio.load = _soundfile_load
    except Exception:
        pass

_patch_torchaudio_load()
