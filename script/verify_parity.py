#!/usr/bin/env python3
import sys
from pathlib import Path
import numpy as np
from scipy.io import wavfile

def load(p):
    sr, x = wavfile.read(p)
    x = x.astype(np.float64) / 32768.0
    return sr, x

def compare(a, b):
    n = min(len(a), len(b)); a, b = a[:n], b[:n]
    rms = np.sqrt(np.mean((a - b) ** 2))
    corr = np.corrcoef(a.reshape(-1), b.reshape(-1))[0, 1]
    return rms, corr

def main():
    py_dir, onnx_dir = Path(sys.argv[1]), Path(sys.argv[2])
    ok = True
    for stem in ["vocals.wav", "no_vocals.wav"]:
        _, a = load(py_dir / stem); _, b = load(onnx_dir / stem)
        rms, corr = compare(a, b)
        status = "PASS" if corr > 0.99 else "FAIL"
        if corr <= 0.99: ok = False
        print(f"{stem:14} rms={rms:.5f} corr={corr:.5f} {status}")
    sys.exit(0 if ok else 1)

if __name__ == "__main__":
    main()
