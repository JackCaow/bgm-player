<script setup lang="ts">
import { computed, watch } from "vue";
import { Icon } from "@iconify/vue";
import { useI18n } from "vue-i18n";
import type { ThemeMode, ModelOption, AudioFormat, AudioQuality, SeparationMode } from "@/types";
import { formatPath } from "@/utils/format";
import { setLocale } from "@/i18n";
import { useExportSettings } from "@/composables/useExportSettings";
import { useSeparationSettings } from "@/composables/useSeparationSettings";
import { useQueueManager } from "@/composables/useQueueManager";

const { t, locale } = useI18n();
const { exportSettings, isLossless, currentBitrate, updateFormat, updateQuality } = useExportSettings();
const { settings: separationSettings, updateMode } = useSeparationSettings();
useQueueManager();

const props = defineProps<{
  theme: ThemeMode;
  model: string;
  outputDir: string;
}>();

const emit = defineEmits<{
  "update:theme": [mode: ThemeMode];
  "update:model": [model: string];
  selectOutputDir: [];
}>();

function normalizeModel(modelValue: string) {
  if (modelValue === "htdemucs") return "htdemucs_onnx";
  if (modelValue === "htdemucs_ft") return "htdemucs_ft_onnx";
  if (modelValue === "htdemucs_6s") return "htdemucs_6s_onnx";
  return modelValue;
}

const activeModel = computed(() => normalizeModel(props.model));

const modelOptions = computed<ModelOption[]>(() => [
  {
    value: "htdemucs_onnx",
    label: "htdemucs (ONNX)",
    desc: t("model.htdemucs_onnx.desc"),
    speed: t("model.speed.medium"),
    quality: t("model.quality.excellent"),
  },
  {
    value: "htdemucs_ft_onnx",
    label: "htdemucs_ft (ONNX)",
    desc: t("model.htdemucs_ft_onnx.desc"),
    speed: t("model.speed.slow"),
    quality: t("model.quality.top"),
  },
  {
    value: "htdemucs_6s_onnx",
    label: "htdemucs_6s (ONNX)",
    desc: t("model.htdemucs_6s_onnx.desc"),
    speed: t("model.speed.slow"),
    quality: t("model.quality.excellent"),
  },
]);

// 获取模型支持的轨道选项
function getAvailableTracks(modelValue: string) {
  if (modelValue === "htdemucs_onnx" || modelValue === "htdemucs_ft_onnx") {
    return [
      { value: "2-track", label: t("separation.twoTrack"), icon: "solar:music-note-2-bold" },
      { value: "4-track", label: t("separation.fourTrack"), icon: "solar:music-notes-bold" },
    ];
  }
  if (modelValue === "htdemucs_6s_onnx") {
    return [
      { value: "2-track", label: t("separation.twoTrack"), icon: "solar:music-note-2-bold" },
      { value: "4-track", label: t("separation.fourTrack"), icon: "solar:music-notes-bold" },
      { value: "6-track", label: t("separation.sixTrack"), icon: "solar:soundwave-bold" },
    ];
  }
  return [
    { value: "2-track", label: t("separation.twoTrack"), icon: "solar:music-note-2-bold" },
  ];
}

const availableTracks = computed(() => getAvailableTracks(activeModel.value));

// 当模型变化时，检查当前轨道模式是否有效，无效则重置
watch(() => props.model, (newModel) => {
  const normalized = normalizeModel(newModel);
  if (normalized !== newModel) {
    emit("update:model", normalized);
  }

  const tracks = getAvailableTracks(normalized);
  const validModes = tracks.map(t => t.value);
  if (!validModes.includes(separationSettings.value.mode)) {
    // 重置为该模型支持的最高轨道数
    updateMode(validModes[validModes.length - 1] as SeparationMode);
  }
});

function changeLocale(lang: string) {
  setLocale(lang);
}
</script>

<template>
  <div class="content-panel">
    <div class="panel-header">
      <h2>{{ t("settings.title") }}</h2>
    </div>

    <div class="settings-content">
      <!-- Model Selection -->
      <div class="setting-group">
        <label class="setting-label">
          <Icon icon="solar:cpu-bolt-bold-duotone" width="20" />
          {{ t("settings.model") }}
        </label>
        <div class="model-list">
          <div
            v-for="opt in modelOptions"
            :key="opt.value"
            class="model-option"
            :class="{ active: activeModel === opt.value }"
            @click="emit('update:model', opt.value)"
          >
            <div class="model-radio">
              <div v-if="activeModel === opt.value" class="radio-dot"></div>
            </div>
            <div class="model-info">
              <span class="model-name">{{ opt.label }}</span>
              <span class="model-desc">{{ opt.desc }}</span>
            </div>
            <div class="model-tags">
              <span class="tag">{{ opt.speed }}</span>
              <span class="tag">{{ opt.quality }}</span>
            </div>
          </div>
        </div>

        <!-- Track Selection (under model) -->
        <div class="track-selection">
          <label class="track-label">
            <Icon icon="solar:music-library-2-bold-duotone" width="18" />
            {{ t("settings.separationMode") }}
          </label>
          <div class="track-options">
            <button
              v-for="track in availableTracks"
              :key="track.value"
              class="track-btn"
              :class="{ active: separationSettings.mode === track.value }"
              @click="updateMode(track.value as any)"
            >
              <Icon :icon="track.icon" width="16" />
              {{ track.label }}
            </button>
          </div>
        </div>
      </div>

      <!-- Output Directory -->
      <div class="setting-group">
        <label class="setting-label">
          <Icon icon="solar:folder-bold-duotone" width="20" />
          {{ t("settings.outputDir") }}
          <span class="optional">{{ t("settings.optional") }}</span>
        </label>
        <div class="output-selector" @click="emit('selectOutputDir')">
          <span v-if="outputDir" class="output-path">{{ formatPath(outputDir) }}</span>
          <span v-else class="output-placeholder">{{ t("settings.defaultOutput") }}</span>
          <Icon icon="solar:alt-arrow-right-linear" width="18" />
        </div>
      </div>

      <!-- Export Format -->
      <div class="setting-group">
        <label class="setting-label">
          <Icon icon="solar:music-library-2-bold-duotone" width="20" />
          {{ t("settings.exportFormat") }}
        </label>
        <div class="format-selector">
          <button
            v-for="fmt in ['wav', 'mp3', 'flac', 'aac', 'm4a', 'ogg', 'opus']"
            :key="fmt"
            class="format-btn"
            :class="{ active: exportSettings.format === fmt }"
            @click="updateFormat(fmt as AudioFormat)"
          >
            <Icon
              :icon="fmt === 'wav' || fmt === 'flac' ? 'solar:diskette-bold' : 'solar:music-note-2-bold'"
              width="16"
            />
            {{ t(`format.${fmt}`) }}
          </button>
        </div>
      </div>

      <!-- Audio Quality -->
      <div class="setting-group">
        <label class="setting-label">
          <Icon icon="solar:soundwave-bold-duotone" width="20" />
          {{ t("settings.audioQuality") }}
          <span v-if="currentBitrate" class="bitrate-info">{{ currentBitrate }} kbps</span>
        </label>
        <div class="quality-selector">
          <button
            v-for="qual in ['low', 'medium', 'high', 'lossless']"
            :key="qual"
            class="quality-btn"
            :class="{
              active: exportSettings.quality === qual,
              disabled: isLossless && qual !== 'lossless'
            }"
            :disabled="isLossless && qual !== 'lossless'"
            @click="updateQuality(qual as AudioQuality)"
          >
            {{ t(`quality.${qual}`) }}
          </button>
        </div>
      </div>

      <!-- Theme -->
      <div class="setting-group">
        <label class="setting-label">
          <Icon icon="solar:palette-bold-duotone" width="20" />
          {{ t("settings.theme") }}
        </label>
        <div class="theme-selector">
          <button
            class="theme-btn"
            :class="{ active: theme === 'light' }"
            @click="emit('update:theme', 'light')"
          >
            <Icon icon="solar:sun-bold" width="18" />
            {{ t("settings.themeLight") }}
          </button>
          <button
            class="theme-btn"
            :class="{ active: theme === 'dark' }"
            @click="emit('update:theme', 'dark')"
          >
            <Icon icon="solar:moon-bold" width="18" />
            {{ t("settings.themeDark") }}
          </button>
          <button
            class="theme-btn"
            :class="{ active: theme === 'system' }"
            @click="emit('update:theme', 'system')"
          >
            <Icon icon="solar:monitor-bold" width="18" />
            {{ t("settings.themeSystem") }}
          </button>
        </div>
      </div>

      <!-- Language -->
      <div class="setting-group">
        <label class="setting-label">
          <Icon icon="solar:global-bold-duotone" width="20" />
          {{ t("settings.language") }}
        </label>
        <div class="lang-selector">
          <button
            class="lang-btn"
            :class="{ active: locale === 'en' }"
            @click="changeLocale('en')"
          >
            English
          </button>
          <button
            class="lang-btn"
            :class="{ active: locale === 'zh' }"
            @click="changeLocale('zh')"
          >
            中文
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Track Selection */
.track-selection {
  margin-top: 16px;
  padding-top: 16px;
  border-top: 1px solid var(--border);
}

.track-label {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 500;
  color: var(--text-secondary);
  margin-bottom: 10px;
}

.track-options {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.track-btn {
  padding: 8px 14px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-card);
  color: var(--text);
  cursor: pointer;
  transition: all 0.2s;
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
}

.track-btn:hover {
  background: var(--bg-hover);
  border-color: var(--primary);
}

.track-btn.active {
  background: var(--primary);
  color: white;
  border-color: var(--primary);
}

.format-selector,
.quality-selector {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.format-btn,
.quality-btn {
  padding: 8px 16px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-card);
  color: var(--text);
  cursor: pointer;
  transition: all 0.2s;
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  white-space: nowrap;
}

.format-btn:hover:not(.disabled),
.quality-btn:hover:not(.disabled) {
  background: var(--bg-hover);
  border-color: var(--primary);
}

.format-btn.active,
.quality-btn.active {
  background: var(--primary);
  color: white;
  border-color: var(--primary);
}

.format-btn.disabled,
.quality-btn.disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.bitrate-info {
  margin-left: auto;
  font-size: 12px;
  color: var(--text-secondary);
  font-weight: normal;
  white-space: nowrap;
  flex-shrink: 0;
}

/* Responsive */
@media (max-width: 768px) {
  .track-btn {
    font-size: 12px;
    padding: 6px 12px;
  }

  .format-btn,
  .quality-btn {
    font-size: 12px;
    padding: 6px 12px;
  }
}

@media (max-width: 480px) {
  .track-btn {
    font-size: 11px;
    padding: 6px 10px;
    gap: 4px;
  }

  .format-btn,
  .quality-btn {
    font-size: 11px;
    padding: 6px 10px;
    gap: 4px;
  }
}

</style>
