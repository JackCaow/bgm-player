<script setup lang="ts">
import { computed, watch } from "vue";
import { Icon } from "@iconify/vue";
import { useI18n } from "vue-i18n";
import type { ThemeMode, ModelOption, AudioFormat, AudioQuality, SeparationMode } from "@/types";
import { formatPath } from "@/utils/format";
import { setLocale } from "@/i18n";
import { useExportSettings } from "@/composables/useExportSettings";
import { useGPUSettings } from "@/composables/useGPUSettings";
import { useSeparationSettings } from "@/composables/useSeparationSettings";
import { useQueueManager } from "@/composables/useQueueManager";

const { t, locale } = useI18n();
const { exportSettings, isLossless, currentBitrate, updateFormat, updateQuality } = useExportSettings();
const { gpuSettings, gpuAvailable, gpuInfo, updateEnabled } = useGPUSettings();
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

const modelOptions = computed<ModelOption[]>(() => [
  {
    value: "htdemucs",
    label: "htdemucs",
    desc: t("model.htdemucs.desc"),
    speed: t("model.speed.fast"),
    quality: t("model.quality.excellent"),
  },
  {
    value: "htdemucs_ft",
    label: "htdemucs_ft",
    desc: t("model.htdemucs_ft.desc"),
    speed: t("model.speed.slow"),
    quality: t("model.quality.top"),
  },
  {
    value: "htdemucs_6s",
    label: "htdemucs_6s",
    desc: t("model.htdemucs_6s.desc"),
    speed: t("model.speed.fast"),
    quality: t("model.quality.excellent"),
  },
  {
    value: "mdx_extra",
    label: "mdx_extra",
    desc: t("model.mdx_extra.desc"),
    speed: t("model.speed.medium"),
    quality: t("model.quality.excellent"),
  },
]);

// 获取模型支持的轨道选项
function getAvailableTracks(modelValue: string) {
  if (modelValue === "htdemucs_6s") {
    return [
      { value: "2-track", label: t("separation.twoTrack"), icon: "solar:music-note-2-bold" },
      { value: "4-track", label: t("separation.fourTrack"), icon: "solar:music-notes-bold" },
      { value: "6-track", label: t("separation.sixTrack"), icon: "solar:soundwave-bold" },
    ];
  }
  if (modelValue === "htdemucs" || modelValue === "htdemucs_ft") {
    return [
      { value: "2-track", label: t("separation.twoTrack"), icon: "solar:music-note-2-bold" },
      { value: "4-track", label: t("separation.fourTrack"), icon: "solar:music-notes-bold" },
    ];
  }
  return [
    { value: "2-track", label: t("separation.twoTrack"), icon: "solar:music-note-2-bold" },
  ];
}

const availableTracks = computed(() => getAvailableTracks(props.model));

// 当模型变化时，检查当前轨道模式是否有效，无效则重置
watch(() => props.model, (newModel) => {
  const tracks = getAvailableTracks(newModel);
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
            :class="{ active: model === opt.value }"
            @click="emit('update:model', opt.value)"
          >
            <div class="model-radio">
              <div v-if="model === opt.value" class="radio-dot"></div>
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

      <!-- GPU Acceleration -->
      <div class="setting-group">
        <label class="setting-label">
          <Icon icon="solar:cpu-bolt-bold-duotone" width="20" />
          GPU 加速
          <span v-if="gpuAvailable" class="gpu-badge available">{{ gpuInfo }}</span>
          <span v-else class="gpu-badge unavailable">不可用</span>
        </label>
        <div class="gpu-toggle">
          <label class="toggle-switch">
            <input
              type="checkbox"
              :checked="gpuSettings.enabled"
              :disabled="!gpuAvailable"
              @change="updateEnabled(($event.target as HTMLInputElement).checked)"
            />
            <span class="toggle-slider"></span>
          </label>
          <span class="toggle-label">
            {{ gpuSettings.enabled ? "已启用" : "已禁用" }}
          </span>
        </div>
        <div v-if="!gpuAvailable" class="gpu-hint">
          <Icon icon="solar:info-circle-linear" width="16" />
          <span>GPU 加速需要 NVIDIA CUDA 或 Apple Metal 支持</span>
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

/* GPU Settings */
.gpu-badge {
  margin-left: auto;
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 600;
}

.gpu-badge.available {
  background: rgba(29, 185, 84, 0.2);
  color: var(--success);
}

.gpu-badge.unavailable {
  background: rgba(255, 164, 43, 0.2);
  color: var(--warning);
}

.gpu-toggle {
  display: flex;
  align-items: center;
  gap: 12px;
}

.toggle-switch {
  position: relative;
  display: inline-block;
  width: 44px;
  height: 24px;
}

.toggle-switch input {
  opacity: 0;
  width: 0;
  height: 0;
}

.toggle-slider {
  position: absolute;
  cursor: pointer;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: var(--bg-card);
  border: 2px solid var(--border);
  transition: 0.3s;
  border-radius: 24px;
}

.toggle-slider:before {
  position: absolute;
  content: "";
  height: 16px;
  width: 16px;
  left: 2px;
  bottom: 2px;
  background-color: var(--text-secondary);
  transition: 0.3s;
  border-radius: 50%;
}

input:checked + .toggle-slider {
  background-color: var(--primary);
  border-color: var(--primary);
}

input:checked + .toggle-slider:before {
  transform: translateX(20px);
  background-color: #000;
}

input:disabled + .toggle-slider {
  opacity: 0.4;
  cursor: not-allowed;
}

.toggle-label {
  font-size: 13px;
  color: var(--text-secondary);
}

.gpu-hint {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 8px;
  padding: 8px 12px;
  background: rgba(255, 164, 43, 0.1);
  border-radius: 6px;
  font-size: 12px;
  color: var(--text-secondary);
}
</style>
