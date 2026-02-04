// Error types
export enum ErrorType {
  FILE_NOT_FOUND = "FILE_NOT_FOUND",
  FILE_FORMAT_UNSUPPORTED = "FILE_FORMAT_UNSUPPORTED",
  PROCESSING_FAILED = "PROCESSING_FAILED",
  FFMPEG_NOT_FOUND = "FFMPEG_NOT_FOUND",
  INSUFFICIENT_SPACE = "INSUFFICIENT_SPACE",
  PERMISSION_DENIED = "PERMISSION_DENIED",
  NETWORK_ERROR = "NETWORK_ERROR",
  UNKNOWN_ERROR = "UNKNOWN_ERROR",
}

// Error details interface
export interface ErrorDetails {
  type: ErrorType;
  message: string;
  suggestion?: string;
  technicalDetails?: string;
  canRetry: boolean;
}

// Parse error from backend
export function parseError(error: unknown): ErrorDetails {
  const errorStr = String(error);

  // File not found
  if (errorStr.includes("文件不存在") || errorStr.includes("not found") || errorStr.includes("No such file")) {
    return {
      type: ErrorType.FILE_NOT_FOUND,
      message: "文件不存在或已被移动",
      suggestion: "请检查文件路径是否正确,或重新选择文件",
      technicalDetails: errorStr,
      canRetry: false,
    };
  }

  // FFmpeg not found
  if (errorStr.includes("ffmpeg") && (errorStr.includes("not found") || errorStr.includes("Make sure ffmpeg is installed"))) {
    return {
      type: ErrorType.FFMPEG_NOT_FOUND,
      message: "FFmpeg 未安装或未找到",
      suggestion: "请安装 FFmpeg: brew install ffmpeg (macOS) 或访问 ffmpeg.org",
      technicalDetails: errorStr,
      canRetry: false,
    };
  }

  // Unsupported format
  if (errorStr.includes("Unsupported format") || errorStr.includes("格式不支持")) {
    return {
      type: ErrorType.FILE_FORMAT_UNSUPPORTED,
      message: "不支持的音频格式",
      suggestion: "请使用 MP3, WAV, FLAC, M4A, OGG, AAC 或 OPUS 格式",
      technicalDetails: errorStr,
      canRetry: false,
    };
  }

  // Permission denied
  if (errorStr.includes("Permission denied") || errorStr.includes("权限") || errorStr.includes("EACCES")) {
    return {
      type: ErrorType.PERMISSION_DENIED,
      message: "没有文件访问权限",
      suggestion: "请检查文件权限,或选择其他输出目录",
      technicalDetails: errorStr,
      canRetry: true,
    };
  }

  // Insufficient space
  if (errorStr.includes("No space left") || errorStr.includes("ENOSPC") || errorStr.includes("空间不足")) {
    return {
      type: ErrorType.INSUFFICIENT_SPACE,
      message: "磁盘空间不足",
      suggestion: "请清理磁盘空间或选择其他输出目录",
      technicalDetails: errorStr,
      canRetry: false,
    };
  }

  // Processing failed
  if (errorStr.includes("处理失败") || errorStr.includes("Failed to convert") || errorStr.includes("Failed to merge")) {
    return {
      type: ErrorType.PROCESSING_FAILED,
      message: "音频处理失败",
      suggestion: "请检查文件是否损坏,或尝试使用其他模型",
      technicalDetails: errorStr,
      canRetry: true,
    };
  }

  // Network error
  if (errorStr.includes("network") || errorStr.includes("ENOTFOUND") || errorStr.includes("ETIMEDOUT")) {
    return {
      type: ErrorType.NETWORK_ERROR,
      message: "网络连接失败",
      suggestion: "请检查网络连接后重试",
      technicalDetails: errorStr,
      canRetry: true,
    };
  }

  // Unknown error
  return {
    type: ErrorType.UNKNOWN_ERROR,
    message: "发生未知错误",
    suggestion: "请重试或联系技术支持",
    technicalDetails: errorStr,
    canRetry: true,
  };
}

// Format error for display
export function formatErrorMessage(error: ErrorDetails): string {
  let message = `❌ ${error.message}`;

  if (error.suggestion) {
    message += `  💡 ${error.suggestion}`;
  }

  if (error.technicalDetails && error.technicalDetails !== error.message) {
    message += `\n\n详细信息: ${error.technicalDetails}`;
  }

  return message;
}

// Get error icon
export function getErrorIcon(type: ErrorType): string {
  switch (type) {
    case ErrorType.FILE_NOT_FOUND:
      return "solar:file-remove-bold-duotone";
    case ErrorType.FILE_FORMAT_UNSUPPORTED:
      return "solar:file-corrupted-bold-duotone";
    case ErrorType.FFMPEG_NOT_FOUND:
      return "solar:settings-minimalistic-bold-duotone";
    case ErrorType.INSUFFICIENT_SPACE:
      return "solar:database-bold-duotone";
    case ErrorType.PERMISSION_DENIED:
      return "solar:lock-bold-duotone";
    case ErrorType.NETWORK_ERROR:
      return "solar:wifi-router-minimalistic-bold-duotone";
    case ErrorType.PROCESSING_FAILED:
      return "solar:danger-triangle-bold-duotone";
    default:
      return "solar:close-circle-bold-duotone";
  }
}
