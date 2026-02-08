#!/usr/bin/env python3
"""
使用 ONNX Runtime 进行 BGM 提取
使用 PyTorch 进行 STFT/iSTFT，ONNX 进行神经网络推理
"""

import argparse
import json
import math
import numpy as np
import torch
import torchaudio
from pathlib import Path


def load_audio(path: str, sample_rate: int = 44100):
    """加载音频文件"""
    audio, sr = torchaudio.load(path)

    # 转换为立体声
    if audio.shape[0] == 1:
        audio = audio.repeat(2, 1)
    elif audio.shape[0] > 2:
        audio = audio[:2]

    # 重采样
    if sr != sample_rate:
        audio = torchaudio.functional.resample(audio, sr, sample_rate)

    return audio


def save_audio(path: str, audio: torch.Tensor, sample_rate: int = 44100):
    """保存音频文件"""
    torchaudio.save(path, audio, sample_rate)


def spectro(x: torch.Tensor, n_fft: int = 4096, hop_length: int = 1024, pad: int = 0):
    """计算 STFT (与 Demucs 相同的实现)"""
    *other, length = x.shape
    x = x.reshape(-1, length)
    z = torch.stft(x,
                   n_fft * (1 + pad),
                   hop_length or n_fft // 4,
                   window=torch.hann_window(n_fft).to(x),
                   win_length=n_fft,
                   normalized=True,
                   center=True,
                   return_complex=True,
                   pad_mode='reflect')
    _, freqs, frame = z.shape
    return z.view(*other, freqs, frame)


def ispectro(z: torch.Tensor, hop_length: int = 1024, length: int = None, pad: int = 0):
    """计算 iSTFT (与 Demucs 相同的实现)"""
    *other, freqs, frames = z.shape
    n_fft = 2 * freqs - 2
    z = z.view(-1, freqs, frames)
    win_length = n_fft // (1 + pad)
    x = torch.istft(z,
                    n_fft,
                    hop_length,
                    window=torch.hann_window(win_length).to(z.real),
                    win_length=win_length,
                    normalized=True,
                    length=length,
                    center=True)
    _, length = x.shape
    return x.view(*other, length)


class OnnxDemucs:
    """ONNX Demucs 推理类"""

    def __init__(self, model_path: str, config_path: str):
        import onnxruntime as ort

        with open(config_path) as f:
            self.config = json.load(f)

        # 使用 CPU 提供程序
        self.session = ort.InferenceSession(
            model_path,
            providers=['CPUExecutionProvider']
        )

        self.sample_rate = self.config["sample_rate"]
        self.channels = self.config["channels"]
        self.sources = self.config["sources"]
        self.nfft = self.config["nfft"]
        self.hop_length = self.config["hop_length"]

        # 获取模型期望的输入形状
        inputs = self.session.get_inputs()
        self.mag_shape = inputs[0].shape
        self.time_shape = inputs[1].shape
        self.segment_samples = self.time_shape[2]
        self.time_frames = self.mag_shape[3]

    def _spec(self, x: torch.Tensor):
        """计算频谱 (与 HTDemucs._spec 相同)"""
        hl = self.hop_length
        nfft = self.nfft

        le = int(math.ceil(x.shape[-1] / hl))
        pad = hl // 2 * 3

        # 使用 reflect padding
        x = torch.nn.functional.pad(x, (pad, pad + le * hl - x.shape[-1]), mode='reflect')

        z = spectro(x, nfft, hl)[..., :-1, :]
        z = z[..., 2: 2 + le]
        return z

    def _ispec(self, z: torch.Tensor, length: int):
        """计算逆频谱 (与 HTDemucs._ispec 相同)"""
        hl = self.hop_length
        z = torch.nn.functional.pad(z, (0, 0, 0, 1))
        z = torch.nn.functional.pad(z, (2, 2))
        pad = hl // 2 * 3
        le = hl * int(math.ceil(length / hl)) + 2 * pad
        x = ispectro(z, hl, length=le)
        x = x[..., pad: pad + length]
        return x

    def _magnitude(self, z: torch.Tensor):
        """将复数频谱转换为实数表示 (cac 模式)"""
        B, C, Fr, T = z.shape
        m = torch.view_as_real(z).permute(0, 1, 4, 2, 3)
        m = m.reshape(B, C * 2, Fr, T)
        return m

    def _mask(self, z: torch.Tensor, m: torch.Tensor):
        """应用掩码 (cac 模式)"""
        B, S, C, Fr, T = m.shape
        out = m.view(B, S, -1, 2, Fr, T).permute(0, 1, 2, 4, 5, 3)
        out = torch.view_as_complex(out.contiguous())
        return out

    def separate(self, audio: torch.Tensor):
        """
        分离音频
        audio: (channels, samples)
        返回: dict of source_name -> (channels, samples)
        """
        original_length = audio.shape[1]

        # 如果音频太长，需要分段处理
        if audio.shape[1] > self.segment_samples:
            return self._separate_long(audio, original_length)

        # 填充到模型期望的长度
        if audio.shape[1] < self.segment_samples:
            pad_length = self.segment_samples - audio.shape[1]
            audio = torch.nn.functional.pad(audio, (0, pad_length))

        # 添加 batch 维度
        audio = audio.unsqueeze(0)  # (1, channels, samples)

        # 计算频谱
        z = self._spec(audio)  # (1, channels, freq, time)

        # 转换为模型输入格式
        mag_input = self._magnitude(z)  # (1, channels*2, freq, time)

        # 确保形状匹配
        target_freq = self.mag_shape[2]
        target_time = self.mag_shape[3]

        if mag_input.shape[2] != target_freq or mag_input.shape[3] != target_time:
            # 调整形状
            if mag_input.shape[2] > target_freq:
                mag_input = mag_input[:, :, :target_freq, :]
                z = z[:, :, :target_freq + 1, :]
            if mag_input.shape[3] > target_time:
                mag_input = mag_input[:, :, :, :target_time]
                z = z[:, :, :, :target_time]
            elif mag_input.shape[3] < target_time:
                pad_time = target_time - mag_input.shape[3]
                mag_input = torch.nn.functional.pad(mag_input, (0, pad_time))
                z = torch.nn.functional.pad(z, (0, pad_time))

        # 转换为 numpy
        mag_np = mag_input.numpy().astype(np.float32)
        time_np = audio.numpy().astype(np.float32)

        # 运行 ONNX 推理
        outputs = self.session.run(None, {
            "mag_input": mag_np,
            "time_input": time_np
        })

        spec_output, time_output = outputs

        # 转换回 PyTorch
        spec_output = torch.from_numpy(spec_output)  # (1, S, C*2, Fr, T)
        time_output = torch.from_numpy(time_output)  # (1, S, C, samples)

        # 应用掩码并转换回时域
        zout = self._mask(z, spec_output)  # (1, S, C, Fr, T)
        x_spec = self._ispec(zout, original_length)  # (1, S, C, samples)

        # 合并频谱和时域输出
        x = x_spec + time_output[..., :original_length]

        # 移除 batch 维度
        x = x.squeeze(0)  # (S, C, samples)

        # 转换为字典
        result = {}
        for i, source_name in enumerate(self.sources):
            result[source_name] = x[i]

        return result

    def _separate_long(self, audio: torch.Tensor, original_length: int):
        """处理长音频（分段处理，与 Demucs 相同的重叠策略）"""
        segment_length = self.segment_samples
        # 使用 1/4 重叠
        stride = segment_length * 3 // 4

        # 计算需要的段数
        num_segments = math.ceil((original_length - segment_length) / stride) + 1

        # 初始化输出
        num_sources = len(self.sources)
        output = torch.zeros(num_sources, self.channels, original_length)
        weight = torch.zeros(1, 1, original_length)

        # 创建窗口函数
        window = torch.ones(segment_length)
        # 使用三角窗口进行平滑过渡
        ramp = torch.linspace(0, 1, segment_length // 4)
        window[:len(ramp)] = ramp
        window[-len(ramp):] = ramp.flip(0)

        for i in range(num_segments):
            start = i * stride
            end = min(start + segment_length, original_length)

            # 提取段
            segment = audio[:, start:end]

            # 填充
            if segment.shape[1] < segment_length:
                segment = torch.nn.functional.pad(segment, (0, segment_length - segment.shape[1]))

            # 分离
            result = self.separate(segment)

            # 计算实际长度
            actual_length = min(segment_length, original_length - start)

            # 应用窗口并累加
            for j, source_name in enumerate(self.sources):
                output[j, :, start:start + actual_length] += result[source_name][:, :actual_length] * window[:actual_length]

            weight[0, 0, start:start + actual_length] += window[:actual_length]

        # 归一化
        output = output / (weight + 1e-8)

        result = {}
        for i, source_name in enumerate(self.sources):
            result[source_name] = output[i]

        return result


def main():
    parser = argparse.ArgumentParser(description="使用 ONNX 进行 BGM 提取")
    parser.add_argument("input", help="输入音频文件")
    parser.add_argument("-o", "--output", default="./output", help="输出目录")
    parser.add_argument("-m", "--model", default="./onnx_models/htdemucs_core.onnx",
                        help="ONNX 模型路径")
    parser.add_argument("-c", "--config", default="./onnx_models/htdemucs_config.json",
                        help="模型配置文件")
    parser.add_argument("--two-stems", type=str, default=None,
                        choices=["vocals", "drums", "bass", "other"],
                        help="只分离两个音轨（指定的和其余的）")

    args = parser.parse_args()

    output_dir = Path(args.output)
    output_dir.mkdir(parents=True, exist_ok=True)

    print(f"加载模型: {args.model}")
    demucs = OnnxDemucs(args.model, args.config)

    print(f"加载音频: {args.input}")
    audio = load_audio(args.input, demucs.sample_rate)
    print(f"音频形状: {audio.shape}, 时长: {audio.shape[1] / demucs.sample_rate:.2f}s")

    print("正在分离...")
    result = demucs.separate(audio)

    input_name = Path(args.input).stem

    if args.two_stems:
        # 只输出两个音轨
        stem = args.two_stems
        other_stems = [s for s in demucs.sources if s != stem]

        # 保存指定音轨
        output_path = output_dir / f"{input_name}_{stem}.wav"
        save_audio(str(output_path), result[stem], demucs.sample_rate)
        print(f"已保存: {output_path}")

        # 保存其余音轨的混合
        no_stem = sum(result[s] for s in other_stems)
        output_path = output_dir / f"{input_name}_no_{stem}.wav"
        save_audio(str(output_path), no_stem, demucs.sample_rate)
        print(f"已保存: {output_path}")
    else:
        # 保存所有音轨
        for stem in demucs.sources:
            output_path = output_dir / f"{input_name}_{stem}.wav"
            save_audio(str(output_path), result[stem], demucs.sample_rate)
            print(f"已保存: {output_path}")

        # 生成 BGM
        if all(s in result for s in ["other", "drums", "bass"]):
            bgm = result["other"] + result["drums"] + result["bass"]
            bgm_path = output_dir / f"{input_name}_bgm.wav"
            save_audio(str(bgm_path), bgm, demucs.sample_rate)
            print(f"已保存 BGM: {bgm_path}")

    print("完成！")


if __name__ == "__main__":
    main()
