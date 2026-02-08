/**
 * Format a timestamp to a relative date string
 */
export function formatDate(timestamp: number, locale: string): string {
  const date = new Date(timestamp);
  const now = new Date();
  const diff = now.getTime() - timestamp;
  const localeCode = locale === "zh" ? "zh-CN" : "en-US";

  // Today
  if (date.toDateString() === now.toDateString()) {
    return date.toLocaleTimeString(localeCode, {
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  // Yesterday
  const yesterday = new Date(now);
  yesterday.setDate(yesterday.getDate() - 1);
  if (date.toDateString() === yesterday.toDateString()) {
    return locale === "zh" ? "昨天" : "Yesterday";
  }

  // Within a week
  if (diff < 7 * 24 * 60 * 60 * 1000) {
    return date.toLocaleDateString(localeCode, {
      weekday: "short",
    });
  }

  // Older
  return date.toLocaleDateString(localeCode, {
    month: "short",
    day: "numeric",
  });
}

/**
 * Format a timestamp to a full date string
 */
export function formatFullDate(timestamp: number, locale: string): string {
  const localeCode = locale === "zh" ? "zh-CN" : "en-US";
  return new Date(timestamp).toLocaleString(localeCode, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

/**
 * Format duration in seconds to a human-readable string
 */
export function formatDuration(seconds: number, locale: string): string {
  if (seconds < 60) {
    return locale === "zh" ? `${seconds} 秒` : `${seconds}s`;
  }
  const mins = Math.floor(seconds / 60);
  const secs = seconds % 60;
  if (locale === "zh") {
    return secs > 0 ? `${mins} 分 ${secs} 秒` : `${mins} 分`;
  }
  return secs > 0 ? `${mins}m ${secs}s` : `${mins}m`;
}

/**
 * Format a file path to a shorter display version
 */
export function formatPath(path: string): string {
  const parts = path.split(/[/\\]/);
  if (parts.length > 3) {
    return ".../" + parts.slice(-2).join("/");
  }
  return path;
}

/**
 * Supported audio file extensions
 */
export const AUDIO_EXTENSIONS = ["mp3", "wav", "flac", "m4a", "ogg", "aac"];

/**
 * Supported video file extensions (for mixing BGM into videos)
 */
export const VIDEO_EXTENSIONS = ["mp4", "mov", "mkv", "webm", "avi", "m4v"];

/**
 * Check if a file path has a supported audio extension
 */
export function isAudioFile(path: string): boolean {
  const ext = path.split(".").pop()?.toLowerCase() || "";
  return AUDIO_EXTENSIONS.includes(ext);
}

/**
 * Check if a file path has a supported video extension
 */
export function isVideoFile(path: string): boolean {
  const ext = path.split(".").pop()?.toLowerCase() || "";
  return VIDEO_EXTENSIONS.includes(ext);
}

/**
 * Extract filename from a path
 */
export function getFileName(path: string): string {
  return path.split("/").pop() || path.split("\\").pop() || path;
}

/**
 * Convert a browser-recorded audio Blob (webm/ogg) to WAV PCM bytes.
 * Uses Web Audio API to decode then re-encode as 16-bit PCM WAV.
 */
export async function blobToWavBytes(blob: Blob): Promise<Uint8Array> {
  const arrayBuffer = await blob.arrayBuffer();
  const audioCtx = new AudioContext();
  try {
    const audioBuffer = await audioCtx.decodeAudioData(arrayBuffer);
    return encodeWav(audioBuffer);
  } finally {
    audioCtx.close();
  }
}

function encodeWav(buffer: AudioBuffer): Uint8Array {
  const numChannels = buffer.numberOfChannels;
  const sampleRate = buffer.sampleRate;
  const bitsPerSample = 16;
  const length = buffer.length;
  const byteRate = sampleRate * numChannels * (bitsPerSample / 8);
  const blockAlign = numChannels * (bitsPerSample / 8);
  const dataSize = length * numChannels * (bitsPerSample / 8);
  const headerSize = 44;
  const out = new ArrayBuffer(headerSize + dataSize);
  const view = new DataView(out);

  // RIFF header
  writeString(view, 0, "RIFF");
  view.setUint32(4, 36 + dataSize, true);
  writeString(view, 8, "WAVE");
  // fmt chunk
  writeString(view, 12, "fmt ");
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true); // PCM
  view.setUint16(22, numChannels, true);
  view.setUint32(24, sampleRate, true);
  view.setUint32(28, byteRate, true);
  view.setUint16(32, blockAlign, true);
  view.setUint16(34, bitsPerSample, true);
  // data chunk
  writeString(view, 36, "data");
  view.setUint32(40, dataSize, true);

  // Interleave channels and write 16-bit PCM
  const channels: Float32Array[] = [];
  for (let ch = 0; ch < numChannels; ch++) {
    channels.push(buffer.getChannelData(ch));
  }
  let offset = headerSize;
  for (let i = 0; i < length; i++) {
    for (let ch = 0; ch < numChannels; ch++) {
      const sample = Math.max(-1, Math.min(1, channels[ch][i]));
      view.setInt16(offset, sample < 0 ? sample * 0x8000 : sample * 0x7FFF, true);
      offset += 2;
    }
  }
  return new Uint8Array(out);
}

function writeString(view: DataView, offset: number, str: string) {
  for (let i = 0; i < str.length; i++) {
    view.setUint8(offset + i, str.charCodeAt(i));
  }
}
