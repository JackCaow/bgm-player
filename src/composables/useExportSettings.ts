import { ref, computed } from "vue";
import type { AudioFormat, AudioQuality, ExportSettings } from "@/types";

const STORAGE_KEY = "export-settings";

// Default export settings
const DEFAULT_SETTINGS: ExportSettings = {
  format: "wav",
  quality: "high",
  sampleRate: 44100,
};

// Bitrate presets for lossy formats
const BITRATE_PRESETS: Record<AudioQuality, number> = {
  low: 128,
  medium: 192,
  high: 256,
  lossless: 320,
};

export function useExportSettings() {
  const exportSettings = ref<ExportSettings>({ ...DEFAULT_SETTINGS });

  const isLossless = computed(() => {
    return exportSettings.value.format === "wav" ||
           exportSettings.value.format === "flac" ||
           exportSettings.value.quality === "lossless";
  });

  const currentBitrate = computed(() => {
    if (isLossless.value) return undefined;
    return exportSettings.value.bitrate || BITRATE_PRESETS[exportSettings.value.quality];
  });

  const formatExtension = computed(() => {
    return `.${exportSettings.value.format}`;
  });

  function loadSettings() {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved) {
        const parsed = JSON.parse(saved);
        exportSettings.value = { ...DEFAULT_SETTINGS, ...parsed };
      }
    } catch (e) {
      console.error("Failed to load export settings:", e);
    }
  }

  function saveSettings() {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(exportSettings.value));
    } catch (e) {
      console.error("Failed to save export settings:", e);
    }
  }

  function updateFormat(format: AudioFormat) {
    exportSettings.value.format = format;

    // Auto-adjust quality for lossless formats
    if (format === "wav" || format === "flac") {
      exportSettings.value.quality = "lossless";
      exportSettings.value.bitrate = undefined;
    } else if (exportSettings.value.quality === "lossless") {
      exportSettings.value.quality = "high";
      exportSettings.value.bitrate = BITRATE_PRESETS.high;
    }

    saveSettings();
  }

  function updateQuality(quality: AudioQuality) {
    exportSettings.value.quality = quality;

    // Update bitrate for lossy formats
    if (!isLossless.value) {
      exportSettings.value.bitrate = BITRATE_PRESETS[quality];
    }

    saveSettings();
  }

  function updateBitrate(bitrate: number) {
    exportSettings.value.bitrate = bitrate;
    saveSettings();
  }

  function updateSampleRate(sampleRate: number) {
    exportSettings.value.sampleRate = sampleRate;
    saveSettings();
  }

  function resetToDefaults() {
    exportSettings.value = { ...DEFAULT_SETTINGS };
    saveSettings();
  }

  // Load settings on initialization
  loadSettings();

  return {
    exportSettings,
    isLossless,
    currentBitrate,
    formatExtension,
    updateFormat,
    updateQuality,
    updateBitrate,
    updateSampleRate,
    resetToDefaults,
  };
}
