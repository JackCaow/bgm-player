// Progress payload from Tauri backend
export interface ProgressPayload {
  progress: number;
  status: string;
  stage: string;
}

// Separation mode types
export type SeparationMode = "2-track" | "4-track" | "6-track";

// Track types for different separation modes
export type TrackType =
  | "vocals"      // 人声
  | "drums"       // 鼓
  | "bass"        // 贝斯
  | "other"       // 其他
  | "guitar"      // 吉他 (仅 6-track)
  | "piano"       // 钢琴 (仅 6-track)
  | "no_vocals";  // BGM (仅 2-track)

// Track information (matches Rust TrackInfo struct)
export interface TrackInfo {
  track_type: TrackType;  // Field name from Rust backend
  path: string;
  name: string;  // Display name
}

// Alias for convenience
export type { TrackType as TrackTypeAlias };

// Extract result with multi-track support
export interface ExtractResult {
  mode: SeparationMode;
  tracks: TrackInfo[];
  // Backward compatibility
  bgmPath?: string;
  vocalsPath?: string;
}

// File item in the processing queue
export interface FileItem {
  id: string;
  path: string;
  name: string;
  status: "pending" | "processing" | "paused" | "cancelled" | "done" | "error";
  progress: number;
  result?: ExtractResult;
  error?: string;
  startTime?: number;
  endTime?: number;
  priority?: number; // Higher number = higher priority
  retryCount?: number;
  estimatedTimeRemaining?: number; // in seconds
  processingTime?: number; // actual processing time in ms
}

// History item for processed files
export interface HistoryItem {
  id: string;
  name: string;
  sourcePath: string;
  mode: SeparationMode;
  tracks: TrackInfo[];
  model: string;
  processedAt: number;
  processingTime?: number;
  sourceSize?: number;
  // Backward compatibility
  bgmPath?: string;
  vocalsPath?: string;
}

// Theme mode type
export type ThemeMode = "light" | "dark" | "system";

// Tab type
export type TabType = "queue" | "result" | "history" | "merge" | "settings";

// Model option for settings
export interface ModelOption {
  value: string;
  label: string;
  desc: string;
  speed: string;
  quality: string;
}

// Audio export formats
export type AudioFormat = "wav" | "mp3" | "flac" | "aac" | "m4a" | "ogg" | "opus";

// Audio quality presets
export type AudioQuality = "low" | "medium" | "high" | "lossless";

// Export settings for audio files
export interface ExportSettings {
  format: AudioFormat;
  quality: AudioQuality;
  bitrate?: number;      // For MP3/AAC: 128, 192, 256, 320 kbps
  sampleRate?: number;   // 44100, 48000 Hz
}

// Audio player state
export interface AudioPlayerState {
  isPlaying: boolean;
  currentTime: number;
  duration: number;
  volume: number;
  activeTrack: TrackType | "all" | null;
}

// Queue settings
export interface QueueSettings {
  maxConcurrent: number; // Maximum concurrent processing tasks
  autoRetry: boolean; // Auto retry failed tasks
  maxRetries: number; // Maximum retry attempts
  pauseOnError: boolean; // Pause queue when error occurs
}

// Queue statistics
export interface QueueStats {
  total: number;
  pending: number;
  processing: number;
  paused: number;
  done: number;
  error: number;
  cancelled: number;
  averageProcessingTime: number; // in seconds
  estimatedTotalTime: number; // in seconds
}

// GPU settings
export interface GPUSettings {
  enabled: boolean;
  device: "cuda" | "mps" | "cpu";
  autoDetect: boolean;
}

// Separation settings
export interface SeparationSettings {
  mode: SeparationMode;
  autoSelectMode: boolean;
}

// Project data structure
export interface Project {
  id: string;
  name: string;
  createdAt: number;
  lastModified: number;
  version: string;

  config: {
    model: string;
    outputDir: string;
    exportSettings: ExportSettings;
    separationSettings: SeparationSettings;
    gpuSettings: GPUSettings;
    queueSettings: QueueSettings;
  };

  files: FileItem[];
  selectedFileId: string | null;

  stats: {
    totalProcessed: number;
    totalFailed: number;
    totalProcessingTime: number;
  };
}
