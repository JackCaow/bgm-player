import { ref } from "vue";

const STORAGE_KEY = "gpu-settings";

export interface GPUSettings {
  enabled: boolean;
  device: "cuda" | "mps" | "cpu";
  autoDetect: boolean;
}

const DEFAULT_SETTINGS: GPUSettings = {
  enabled: false,
  device: "cpu",
  autoDetect: true,
};

export function useGPUSettings() {
  const gpuSettings = ref<GPUSettings>({ ...DEFAULT_SETTINGS });
  const gpuAvailable = ref(false);
  const gpuInfo = ref<string>("");

  function loadSettings() {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved) {
        const parsed = JSON.parse(saved);
        gpuSettings.value = { ...DEFAULT_SETTINGS, ...parsed };
      }
    } catch (e) {
      console.error("Failed to load GPU settings:", e);
    }
  }

  function saveSettings() {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(gpuSettings.value));
    } catch (e) {
      console.error("Failed to save GPU settings:", e);
    }
  }

  function updateEnabled(enabled: boolean) {
    gpuSettings.value.enabled = enabled;
    saveSettings();
  }

  function updateDevice(device: GPUSettings["device"]) {
    gpuSettings.value.device = device;
    saveSettings();
  }

  function updateAutoDetect(autoDetect: boolean) {
    gpuSettings.value.autoDetect = autoDetect;
    saveSettings();
  }

  async function detectGPU() {
    // Check for CUDA (NVIDIA)
    if (navigator.userAgent.includes("CUDA") || await checkCUDA()) {
      gpuAvailable.value = true;
      gpuInfo.value = "NVIDIA CUDA";
      if (gpuSettings.value.autoDetect) {
        gpuSettings.value.device = "cuda";
      }
      return;
    }

    // Check for Metal (Apple Silicon)
    if (navigator.userAgent.includes("Mac") && await checkMetal()) {
      gpuAvailable.value = true;
      gpuInfo.value = "Apple Metal";
      if (gpuSettings.value.autoDetect) {
        gpuSettings.value.device = "mps";
      }
      return;
    }

    // Fallback to CPU
    gpuAvailable.value = false;
    gpuInfo.value = "CPU only";
    gpuSettings.value.device = "cpu";
    gpuSettings.value.enabled = false;
  }

  async function checkCUDA(): Promise<boolean> {
    // This is a placeholder - actual CUDA detection would require backend support
    return false;
  }

  async function checkMetal(): Promise<boolean> {
    // Check if running on Apple Silicon
    return navigator.userAgent.includes("Mac") && navigator.hardwareConcurrency > 4;
  }

  // Load settings on initialization
  loadSettings();
  detectGPU();

  return {
    gpuSettings,
    gpuAvailable,
    gpuInfo,
    updateEnabled,
    updateDevice,
    updateAutoDetect,
    detectGPU,
  };
}
