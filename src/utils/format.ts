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
 * Check if a file path has a supported audio extension
 */
export function isAudioFile(path: string): boolean {
  const ext = path.split(".").pop()?.toLowerCase() || "";
  return AUDIO_EXTENSIONS.includes(ext);
}

/**
 * Extract filename from a path
 */
export function getFileName(path: string): string {
  return path.split("/").pop() || path.split("\\").pop() || path;
}
