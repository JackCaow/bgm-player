import { ref } from "vue";

const STORAGE_KEY = "onnx-settings";

export interface OnnxSettings {
  enabled: boolean;
  executionProvider: "cpu" | "coreml" | "cuda";
}

const DEFAULT_SETTINGS: OnnxSettings = {
  enabled: false,
  executionProvider: "cpu",
};

export function useOnnxSettings() {
  const onnxSettings = ref<OnnxSettings>({ ...DEFAULT_SETTINGS });
  const onnxAvailable = ref(true); // Assume available, will be validated on first use

  function loadSettings() {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved) {
        const parsed = JSON.parse(saved);
        onnxSettings.value = { ...DEFAULT_SETTINGS, ...parsed };
      }
    } catch (e) {
      console.error("Failed to load ONNX settings:", e);
    }
  }

  function saveSettings() {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(onnxSettings.value));
    } catch (e) {
      console.error("Failed to save ONNX settings:", e);
    }
  }

  function updateEnabled(enabled: boolean) {
    onnxSettings.value.enabled = enabled;
    saveSettings();
  }

  function updateExecutionProvider(provider: OnnxSettings["executionProvider"]) {
    onnxSettings.value.executionProvider = provider;
    saveSettings();
  }

  function setOnnxAvailable(available: boolean) {
    onnxAvailable.value = available;
    if (!available) {
      onnxSettings.value.enabled = false;
      saveSettings();
    }
  }

  // Load settings on initialization
  loadSettings();

  return {
    onnxSettings,
    onnxAvailable,
    updateEnabled,
    updateExecutionProvider,
    setOnnxAvailable,
  };
}
