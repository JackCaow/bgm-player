#!/usr/bin/env python3
"""
导出 Demucs 模型为 ONNX 格式
使用自定义的 MultiheadAttention 实现来避免 ONNX 导出问题
"""

import torch
import torch.nn as nn
import torch.nn.functional as F
import argparse
from pathlib import Path
import math
from einops import rearrange


class OnnxCompatibleMultiheadAttention(nn.Module):
    """ONNX 兼容的 MultiheadAttention 实现"""

    def __init__(self, embed_dim, num_heads, dropout=0.0, batch_first=False):
        super().__init__()
        self.embed_dim = embed_dim
        self.num_heads = num_heads
        self.head_dim = embed_dim // num_heads
        self.batch_first = batch_first

        self.q_proj = nn.Linear(embed_dim, embed_dim)
        self.k_proj = nn.Linear(embed_dim, embed_dim)
        self.v_proj = nn.Linear(embed_dim, embed_dim)
        self.out_proj = nn.Linear(embed_dim, embed_dim)
        self.dropout = nn.Dropout(dropout)

    def forward(self, query, key, value, key_padding_mask=None, need_weights=False,
                attn_mask=None, average_attn_weights=True, is_causal=False):
        # 如果不是 batch_first，转换为 batch_first
        if not self.batch_first:
            query = query.transpose(0, 1)
            key = key.transpose(0, 1)
            value = value.transpose(0, 1)

        B, T, C = query.shape
        _, S, _ = key.shape

        # 投影
        q = self.q_proj(query).view(B, T, self.num_heads, self.head_dim).transpose(1, 2)
        k = self.k_proj(key).view(B, S, self.num_heads, self.head_dim).transpose(1, 2)
        v = self.v_proj(value).view(B, S, self.num_heads, self.head_dim).transpose(1, 2)

        # 缩放点积注意力
        scale = 1.0 / math.sqrt(self.head_dim)
        attn_weights = torch.matmul(q, k.transpose(-2, -1)) * scale

        if attn_mask is not None:
            attn_weights = attn_weights + attn_mask

        attn_weights = F.softmax(attn_weights, dim=-1)
        attn_weights = self.dropout(attn_weights)

        # 应用注意力
        out = torch.matmul(attn_weights, v)
        out = out.transpose(1, 2).contiguous().view(B, T, C)
        out = self.out_proj(out)

        # 如果不是 batch_first，转换回来
        if not self.batch_first:
            out = out.transpose(0, 1)

        return out, None


def replace_multihead_attention(module):
    """递归替换所有 nn.MultiheadAttention 为 ONNX 兼容版本"""
    for name, child in module.named_children():
        if isinstance(child, nn.MultiheadAttention):
            # 创建新的 ONNX 兼容版本
            new_attn = OnnxCompatibleMultiheadAttention(
                embed_dim=child.embed_dim,
                num_heads=child.num_heads,
                dropout=child.dropout,
                batch_first=child.batch_first
            )
            # 复制权重
            with torch.no_grad():
                # nn.MultiheadAttention 使用 in_proj_weight 和 in_proj_bias
                if child.in_proj_weight is not None:
                    # 分割 in_proj_weight
                    q_weight, k_weight, v_weight = child.in_proj_weight.chunk(3, dim=0)
                    new_attn.q_proj.weight.copy_(q_weight)
                    new_attn.k_proj.weight.copy_(k_weight)
                    new_attn.v_proj.weight.copy_(v_weight)
                if child.in_proj_bias is not None:
                    q_bias, k_bias, v_bias = child.in_proj_bias.chunk(3, dim=0)
                    new_attn.q_proj.bias.copy_(q_bias)
                    new_attn.k_proj.bias.copy_(k_bias)
                    new_attn.v_proj.bias.copy_(v_bias)
                # 复制输出投影权重
                new_attn.out_proj.weight.copy_(child.out_proj.weight)
                new_attn.out_proj.bias.copy_(child.out_proj.bias)

            setattr(module, name, new_attn)
        else:
            replace_multihead_attention(child)


class DemucsCore(nn.Module):
    """HTDemucs 核心神经网络"""

    def __init__(self, model):
        super().__init__()
        self.model = model
        self.sources = model.sources
        self.audio_channels = model.audio_channels

    def forward(self, mag_input, time_input):
        x = mag_input
        xt = time_input

        B, _, Fq, T = x.shape
        S = len(self.sources)

        # 归一化
        mean = x.mean(dim=(1, 2, 3), keepdim=True)
        std = x.std(dim=(1, 2, 3), keepdim=True)
        x = (x - mean) / (1e-5 + std)

        meant = xt.mean(dim=(1, 2), keepdim=True)
        stdt = xt.std(dim=(1, 2), keepdim=True)
        xt = (xt - meant) / (1e-5 + stdt)

        # 编码器
        saved = []
        saved_t = []
        lengths = []
        lengths_t = []

        for idx, encode in enumerate(self.model.encoder):
            lengths.append(x.shape[-1])
            inject = None
            if idx < len(self.model.tencoder):
                lengths_t.append(xt.shape[-1])
                tenc = self.model.tencoder[idx]
                xt = tenc(xt)
                if not tenc.empty:
                    saved_t.append(xt)
                else:
                    inject = xt
            x = encode(x, inject)
            if idx == 0 and self.model.freq_emb is not None:
                frs = torch.arange(x.shape[-2], device=x.device)
                emb = self.model.freq_emb(frs).t()[None, :, :, None].expand_as(x)
                x = x + self.model.freq_emb_scale * emb
            saved.append(x)

        # Cross Transformer
        if self.model.crosstransformer:
            if self.model.bottom_channels:
                b, c, f, t = x.shape
                x = x.reshape(b, c, f * t)
                x = self.model.channel_upsampler(x)
                x = x.reshape(b, -1, f, t)
                xt = self.model.channel_upsampler_t(xt)

            x, xt = self.model.crosstransformer(x, xt)

            if self.model.bottom_channels:
                x = x.reshape(b, -1, f * t)
                x = self.model.channel_downsampler(x)
                x = x.reshape(b, -1, f, t)
                xt = self.model.channel_downsampler_t(xt)

        # 解码器
        for idx, decode in enumerate(self.model.decoder):
            skip = saved.pop(-1)
            x, pre = decode(x, skip, lengths.pop(-1))

            offset = self.model.depth - len(self.model.tdecoder)
            if idx >= offset:
                tdec = self.model.tdecoder[idx - offset]
                length_t = lengths_t.pop(-1)
                if tdec.empty:
                    pre = pre[:, :, 0]
                    xt, _ = tdec(pre, None, length_t)
                else:
                    skip = saved_t.pop(-1)
                    xt, _ = tdec(xt, skip, length_t)

        # 输出处理
        x = x.view(B, S, -1, Fq, T)
        x = x * std[:, None] + mean[:, None]

        length = time_input.shape[-1]
        xt = xt.view(B, S, -1, length)
        xt = xt * stdt[:, None] + meant[:, None]

        return x, xt


def export_demucs_to_onnx(model_name: str, output_dir: Path):
    """导出 Demucs 模型为 ONNX"""
    print(f"正在加载模型: {model_name}")

    from demucs.pretrained import get_model

    bag_model = get_model(model_name)
    bag_model.eval()

    num_models = len(bag_model.models)
    model0 = bag_model.models[0]
    model0.eval()

    sample_rate = model0.samplerate
    channels = model0.audio_channels
    sources = model0.sources
    nfft = model0.nfft
    hop_length = model0.hop_length

    print(f"采样率: {sample_rate}")
    print(f"声道数: {channels}")
    print(f"输出源: {sources}")
    print(f"NFFT: {nfft}, Hop: {hop_length}")
    if num_models > 1:
        print(f"检测到 BagOfModels（{num_models} 个子模型），将分别导出并在配置中写入 ensemble 权重...")

    # 计算频谱维度
    freq_bins = nfft // 2 + 1
    segment_samples = int(sample_rate * 7.8)
    time_frames = int(math.ceil(segment_samples / hop_length))

    print(f"频率bins: {freq_bins}, 时间帧: {time_frames}")

    # 创建示例输入
    dummy_mag = torch.randn(1, channels * 2, freq_bins - 1, time_frames)
    dummy_time = torch.randn(1, channels, segment_samples)

    print(f"频谱输入形状: {dummy_mag.shape}")
    print(f"时域输入形状: {dummy_time.shape}")

    def export_single(sub_model: torch.nn.Module, output_path: Path):
        sub_model.eval()
        print("替换 MultiheadAttention 为 ONNX 兼容版本...")
        replace_multihead_attention(sub_model)

        # 创建核心模型包装器
        core_model = DemucsCore(sub_model)
        core_model.eval()

        print(f"正在导出 ONNX 模型到: {output_path}")
        print("正在追踪模型...")
        with torch.no_grad():
            out_spec, out_time = core_model(dummy_mag, dummy_time)
            print(f"频谱输出形状: {out_spec.shape}")
            print(f"时域输出形状: {out_time.shape}")

        print("正在使用 TorchScript 追踪模型...")
        traced_model = torch.jit.trace(
            core_model,
            (dummy_mag, dummy_time),
            check_trace=False,
            strict=False
        )

        print("正在导出 ONNX...")
        torch.onnx.export(
            traced_model,
            (dummy_mag, dummy_time),
            str(output_path),
            input_names=["mag_input", "time_input"],
            output_names=["spec_output", "time_output"],
            opset_version=14,
            do_constant_folding=True,
            verbose=False,
            export_params=True,
            dynamo=False
        )
        print(f"导出成功！文件大小: {output_path.stat().st_size / 1024 / 1024:.1f} MB")

    try:
        exported_filenames = []
        if num_models == 1:
            output_path = output_dir / f"{model_name}_core.onnx"
            export_single(bag_model.models[0], output_path)
            exported_filenames = [output_path.name]
        else:
            for idx, sub_model in enumerate(bag_model.models):
                output_path = output_dir / f"{model_name}_core_{idx}.onnx"
                export_single(sub_model, output_path)
                exported_filenames.append(output_path.name)

        # 保存模型配置（含 ensemble 权重）
        config_path = output_dir / f"{model_name}_config.json"
        import json
        config = {
            "model_name": model_name,
            "sample_rate": sample_rate,
            "channels": channels,
            "sources": sources,
            "nfft": nfft,
            "hop_length": hop_length,
            "freq_bins": freq_bins
        }
        if num_models > 1:
            config["ensemble"] = {
                "models": exported_filenames,
                "weights": bag_model.weights
            }
        with open(config_path, "w") as f:
            json.dump(config, f, indent=2)
        print(f"配置已保存到: {config_path}")

        return True
    except Exception as e:
        print(f"导出失败: {e}")
        import traceback
        traceback.print_exc()
        return False


def verify_onnx_model(onnx_path: Path):
    """验证 ONNX 模型"""
    print(f"\n正在验证 ONNX 模型: {onnx_path}")

    try:
        import onnx
        model = onnx.load(str(onnx_path))
        onnx.checker.check_model(model)
        print("ONNX 模型验证通过！")

        print(f"输入: {[i.name for i in model.graph.input]}")
        print(f"输出: {[o.name for o in model.graph.output]}")
        return True
    except Exception as e:
        print(f"验证失败: {e}")
        return False


def test_onnx_inference(onnx_path: Path, config_path: Path):
    """测试 ONNX 推理"""
    print(f"\n正在测试 ONNX 推理...")

    try:
        import onnxruntime as ort
        import numpy as np
        import json

        with open(config_path) as f:
            config = json.load(f)

        session = ort.InferenceSession(str(onnx_path))

        # 获取模型期望的输入形状
        inputs = session.get_inputs()
        for inp in inputs:
            print(f"模型输入: {inp.name}, 形状: {inp.shape}")

        # 使用模型期望的固定形状
        mag_shape = inputs[0].shape
        time_shape = inputs[1].shape

        mag_input = np.random.randn(*mag_shape).astype(np.float32)
        time_input = np.random.randn(*time_shape).astype(np.float32)

        outputs = session.run(None, {
            "mag_input": mag_input,
            "time_input": time_input
        })

        print(f"推理成功！")
        print(f"频谱输入形状: {mag_input.shape}")
        print(f"时域输入形状: {time_input.shape}")
        print(f"频谱输出形状: {outputs[0].shape}")
        print(f"时域输出形状: {outputs[1].shape}")
        return True
    except Exception as e:
        print(f"推理失败: {e}")
        import traceback
        traceback.print_exc()
        return False


def main():
    parser = argparse.ArgumentParser(description="导出 Demucs 模型为 ONNX")
    parser.add_argument(
        "-m", "--model",
        default="htdemucs",
        choices=["htdemucs", "htdemucs_ft", "htdemucs_6s", "mdx_extra"],
        help="模型名称"
    )
    parser.add_argument(
        "-o", "--output",
        default="./onnx_models",
        help="输出目录"
    )
    parser.add_argument(
        "--verify",
        action="store_true",
        help="验证导出的模型"
    )
    parser.add_argument(
        "--test",
        action="store_true",
        help="测试推理"
    )

    args = parser.parse_args()

    output_dir = Path(args.output)
    output_dir.mkdir(parents=True, exist_ok=True)

    success = export_demucs_to_onnx(args.model, output_dir)

    if success:
        onnx_path = output_dir / f"{args.model}_core.onnx"
        if not onnx_path.exists():
            onnx_path = output_dir / f"{args.model}_core_0.onnx"
        config_path = output_dir / f"{args.model}_config.json"

        if args.verify:
            verify_onnx_model(onnx_path)

        if args.test:
            test_onnx_inference(onnx_path, config_path)


if __name__ == "__main__":
    main()
