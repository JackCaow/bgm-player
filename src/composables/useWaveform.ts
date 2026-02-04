import { ref, onUnmounted } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";

export function useWaveform() {
  const audioContext = ref<AudioContext | null>(null);

  async function drawWaveform(
    audioPath: string,
    canvas: HTMLCanvasElement | null
  ) {
    if (!canvas) {
      console.error("[Waveform] Canvas is null");
      return;
    }

    const ctx = canvas.getContext("2d");
    if (!ctx) {
      console.error("[Waveform] Cannot get 2d context");
      return;
    }

    // Get the actual display size
    const rect = canvas.getBoundingClientRect();
    const width = rect.width || 400;
    const height = rect.height || 60;

    // Set canvas internal size to match display size (for high DPI)
    const dpr = window.devicePixelRatio || 1;
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    ctx.scale(dpr, dpr);

    console.log("[Waveform] Canvas rect:", width, "x", height, "dpr:", dpr);

    // Clear canvas
    ctx.clearRect(0, 0, width, height);

    // Draw background
    ctx.fillStyle = "rgba(29, 185, 84, 0.15)";
    ctx.fillRect(0, 0, width, height);

    try {
      if (!audioContext.value) {
        audioContext.value = new AudioContext();
      }

      // Use Tauri's convertFileSrc for proper file access
      const src = convertFileSrc(audioPath);
      console.log("[Waveform] Loading audio from:", audioPath);
      console.log("[Waveform] Converted src:", src);

      const response = await fetch(src);
      if (!response.ok) {
        console.error("[Waveform] Fetch failed:", response.status, response.statusText);
        return;
      }

      const arrayBuffer = await response.arrayBuffer();
      console.log("[Waveform] ArrayBuffer size:", arrayBuffer.byteLength);

      console.log("[Waveform] Decoding audio data...");
      const audioBuffer = await audioContext.value.decodeAudioData(arrayBuffer);
      console.log("[Waveform] Audio decoded, duration:", audioBuffer.duration, "channels:", audioBuffer.numberOfChannels);

      const data = audioBuffer.getChannelData(0);
      const drawWidth = rect.width;
      const drawHeight = rect.height;
      const step = Math.ceil(data.length / drawWidth);
      const amp = drawHeight / 2;

      console.log("[Waveform] Drawing waveform, data length:", data.length, "step:", step, "canvas size:", drawWidth, "x", drawHeight);

      ctx.beginPath();
      ctx.moveTo(0, amp);
      ctx.strokeStyle = "#1DB954";
      ctx.lineWidth = 2;

      for (let i = 0; i < drawWidth; i++) {
        let min = 1.0;
        let max = -1.0;
        for (let j = 0; j < step; j++) {
          const datum = data[i * step + j];
          if (datum < min) min = datum;
          if (datum > max) max = datum;
        }
        ctx.lineTo(i, (1 + min) * amp);
        ctx.lineTo(i, (1 + max) * amp);
      }

      ctx.stroke();
      console.log("[Waveform] Waveform drawn successfully");
    } catch (e) {
      console.error("Failed to draw waveform:", e);
    }
  }

  function cleanup() {
    audioContext.value?.close();
  }

  onUnmounted(() => {
    cleanup();
  });

  return {
    drawWaveform,
  };
}
