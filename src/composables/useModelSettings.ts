import { ref, watch } from "vue";

const STORAGE_KEY = "model-settings";

// Global state
const model = ref<string>("htdemucs_onnx");

function normalizeModel(modelValue: string) {
  if (modelValue === "htdemucs") return "htdemucs_onnx";
  if (modelValue === "htdemucs_ft") return "htdemucs_ft_onnx";
  if (modelValue === "htdemucs_6s") return "htdemucs_6s_onnx";
  return modelValue;
}

// Load settings from localStorage
function loadSettings() {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      model.value = normalizeModel(stored);
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
