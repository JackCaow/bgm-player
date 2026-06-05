import { ref, watch } from "vue";

const STORAGE_KEY = "model-settings";

// Global state
const model = ref<string>("htdemucs");

// 打包版仅支持 htdemucs(ONNX 路径)。其它模型入口已从 UI 移除;
// 忽略任何遗留的持久化值,回落到 htdemucs。
const SUPPORTED_MODELS = ["htdemucs"];

// Load settings from localStorage
function loadSettings() {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored && SUPPORTED_MODELS.includes(stored)) {
      model.value = stored;
    }
  } catch (error) {
    console.error("Failed to load model settings:", error);
  }
}

// Save settings to localStorage
function saveSettings() {
  try {
    localStorage.setItem(STORAGE_KEY, model.value);
  } catch (error) {
    console.error("Failed to save model settings:", error);
  }
}

// Initialize on first import
loadSettings();

// Auto-save on changes
watch(model, saveSettings);

export function useModelSettings() {
  function updateModel(newModel: string) {
    model.value = newModel;
  }

  return {
    model,
    updateModel,
  };
}
