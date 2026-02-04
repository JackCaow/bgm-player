import { ref, watch } from "vue";
import type { SeparationMode } from "../types";

export interface SeparationSettings {
  mode: SeparationMode;
}

const STORAGE_KEY = "separation-settings";

// Global state
const settings = ref<SeparationSettings>({
  mode: "2-track",
});

// Load settings from localStorage
function loadSettings() {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      const parsed = JSON.parse(stored);
      settings.value = { ...settings.value, ...parsed };
    }
  } catch (error) {
    console.error("Failed to load separation settings:", error);
  }
}

// Save settings to localStorage
function saveSettings() {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings.value));
  } catch (error) {
    console.error("Failed to save separation settings:", error);
  }
}

// Initialize on first import
loadSettings();

// Auto-save on changes
watch(settings, saveSettings, { deep: true });

export function useSeparationSettings() {
  // 更新分离模式
  function updateMode(mode: SeparationMode) {
    settings.value.mode = mode;
  }

  // 获取当前分离模式
  function getEffectiveMode(_model: string): SeparationMode {
    return settings.value.mode;
  }

  return {
    settings,
    updateMode,
    getEffectiveMode,
  };
}

