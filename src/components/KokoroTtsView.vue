<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { useI18n } from "vue-i18n";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import Slider from "@/components/ui/Slider.vue";
import AudioPlayer from "@/components/AudioPlayer.vue";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { formatPath } from "@/utils/format";

const { t } = useI18n();

defineProps<{
  text: string;
  ttsEngine: string;
  voice: string;
  language: string;
  speed: number;
  outputDir: string;
  bgmPath: string;
  bgmVolume: number;
  ttsVolume: number;
  loopBgm: boolean;
  isGenerating: boolean;
  isMerging: boolean;
  outputPath: string | null;
  dryOutputPath: string | null;
  canGenerate: boolean;
  canMerge: boolean;
  refAudioPath: string;
  refText: string;
  isRecording: boolean;
  recordingSeconds: number;
}>();

const emit = defineEmits<{
  "update:text": [value: string];
  "update:ttsEngine": [value: string];
  "update:voice": [value: string];
  "update:language": [value: string];
  "update:speed": [value: number];
  "update:bgmVolume": [value: number];
  "update:ttsVolume": [value: number];
  "update:loopBgm": [value: boolean];
  selectOutputDir: [];
  selectBgm: [];
  selectRefAudio: [];
  startRecording: [];
  stopRecording: [];
  "update:refText": [value: string];
  generate: [];
  merge: [];
  clear: [];
}>();

const ttsEngineOptions = [
  { value: "kokoro", label: "Kokoro" },
];

const kokoroVoiceOptions = [
  { value: "af_heart", label: "af_heart" },
  { value: "af_bella", label: "af_bella" },
  { value: "am_adam", label: "am_adam" },
  { value: "bf_emma", label: "bf_emma" },
  { value: "bm_george", label: "bm_george" },
];

const languageOptions = [
  { value: "en-us", label: "English" },
  { value: "zh", label: "中文" },
  { value: "ja", label: "日本語" },
];
</script>

<template>
  <div class="content-panel">
    <div class="panel-header">
      <h2>{{ t("tts.title") }}</h2>
      <Button v-if="text || outputPath" variant="outline" size="sm" @click="emit('clear')">
        {{ t("tts.clear") }}
      </Button>
    </div>

    <div class="tts-content">
      <section class="tts-card">
        <div class="tts-step-title">{{ t("tts.stepGenerate") }}</div>
        <div class="tts-hint">{{ t("tts.emptyHint") }}</div>

        <label class="tts-label">{{ t("tts.text") }}</label>
        <textarea
          :value="text"
          class="tts-textarea"
          :placeholder="t('tts.textPlaceholder')"
          rows="7"
          @input="emit('update:text', ($event.target as HTMLTextAreaElement).value)"
        />

        <div class="tts-grid">
          <div>
            <label class="tts-label">{{ t("tts.engine") }}</label>
            <Select :model-value="ttsEngine" @update:modelValue="emit('update:ttsEngine', String($event))">
              <SelectTrigger class="w-full">
                <SelectValue :placeholder="t('tts.engine')" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem v-for="item in ttsEngineOptions" :key="item.value" :value="item.value">
                  {{ item.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div>
            <label class="tts-label">{{ t("tts.voice") }}</label>
            <Select :model-value="voice" @update:modelValue="emit('update:voice', String($event))">
              <SelectTrigger class="w-full">
                <SelectValue :placeholder="t('tts.voice')" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem
                  v-for="item in kokoroVoiceOptions"
                  :key="item.value"
                  :value="item.value"
                >
                  {{ item.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div>
            <label class="tts-label">{{ t("tts.language") }}</label>
            <Select :model-value="language" @update:modelValue="emit('update:language', String($event))">
              <SelectTrigger class="w-full">
                <SelectValue :placeholder="t('tts.language')" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem v-for="item in languageOptions" :key="item.value" :value="item.value">
                  {{ item.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>

        <div class="tts-speed-wrap">
          <div class="tts-speed-header">
            <label class="tts-label">{{ t("tts.speed") }}</label>
            <span class="tts-speed-value">{{ speed.toFixed(2) }}x</span>
          </div>
          <Slider
            :model-value="speed"
            :min="0.6"
            :max="1.6"
            :step="0.05"
            @update:model-value="emit('update:speed', $event)"
          />
        </div>

        <div class="tts-output" @click="emit('selectOutputDir')">
          <div class="tts-output-left">
            <div class="tts-label">{{ t("tts.outputDir") }}</div>
            <div class="tts-output-path" :class="{ empty: !outputDir }">
              {{ outputDir ? formatPath(outputDir) : t("tts.outputDirOptional") }}
            </div>
          </div>
          <Icon icon="solar:alt-arrow-right-linear" width="16" class="tts-output-arrow" />
        </div>
        <Button class="mt-4 h-11 w-full rounded-xl" :disabled="!canGenerate" @click="emit('generate')">
          <template v-if="!isGenerating">
            <Icon icon="solar:microphone-3-bold" width="18" />
            {{ t("tts.start") }}
          </template>
          <template v-else>
            <Icon icon="solar:refresh-bold" width="18" class="animate-spin" />
            {{ t("tts.generating") }}
          </template>
        </Button>
      </section>

      <section class="tts-card" :class="{ 'tts-card-disabled': !dryOutputPath }">
        <div class="tts-step-title">{{ t("tts.stepMix") }}</div>
        <div class="tts-hint">{{ dryOutputPath ? t("tts.mixHint") : t("tts.mixLockedHint") }}</div>

        <div
          class="tts-output"
          :class="{ disabled: !dryOutputPath }"
          @click="dryOutputPath && emit('selectBgm')"
        >
          <div class="tts-output-left">
            <div class="tts-label">{{ t("tts.bgm") }}</div>
            <div class="tts-output-path" :class="{ empty: !bgmPath }">
              {{ bgmPath ? formatPath(bgmPath) : t("tts.selectBgm") }}
            </div>
          </div>
          <Icon icon="solar:alt-arrow-right-linear" width="16" class="tts-output-arrow" />
        </div>

        <div class="tts-speed-wrap">
          <div class="tts-speed-header">
            <label class="tts-label">{{ t("tts.bgmVolume") }}</label>
            <span class="tts-speed-value">{{ Math.round(bgmVolume * 100) }}%</span>
          </div>
          <Slider
            :model-value="bgmVolume"
            :min="0"
            :max="1"
            :step="0.02"
            :disabled="!dryOutputPath"
            @update:model-value="emit('update:bgmVolume', $event)"
          />
        </div>

        <div class="tts-speed-wrap">
          <div class="tts-speed-header">
            <label class="tts-label">{{ t("tts.vocalsVolume") }}</label>
            <span class="tts-speed-value">{{ Math.round(ttsVolume * 100) }}%</span>
          </div>
          <Slider
            :model-value="ttsVolume"
            :min="0"
            :max="2"
            :step="0.02"
            :disabled="!dryOutputPath"
            @update:model-value="emit('update:ttsVolume', $event)"
          />
        </div>

        <div class="tts-loop-toggle">
          <Checkbox
            :model-value="loopBgm"
            :disabled="!dryOutputPath"
            @update:model-value="emit('update:loopBgm', Boolean($event))"
          />
          <span class="tts-loop-label">
            {{ t("tts.loopBgm") }}: {{ loopBgm ? t("tts.on") : t("tts.off") }}
          </span>
        </div>

        <Button
          class="mt-4 h-11 w-full rounded-xl"
          variant="outline"
          :disabled="!canMerge"
          @click="emit('merge')"
        >
          <template v-if="!isMerging">
            <Icon icon="solar:layers-bold" width="18" />
            {{ t("tts.merge") }}
          </template>
          <template v-else>
            <Icon icon="solar:refresh-bold" width="18" class="animate-spin" />
            {{ t("tts.merging") }}
          </template>
        </Button>
      </section>

      <section v-if="outputPath" class="tts-card">
        <div class="result-header">
          <Icon icon="solar:check-circle-bold" width="20" class="text-primary" />
          <span class="result-title">{{ t("tts.result") }}</span>
        </div>
        <div class="result-path">{{ outputPath }}</div>
        <audio controls :src="'asset://localhost/' + outputPath" class="w-full"></audio>
      </section>

      <section v-if="dryOutputPath && bgmPath" class="tts-card">
        <div class="result-header">
          <Icon icon="solar:music-note-slider-bold" width="20" class="text-primary" />
          <span class="result-title">{{ t("tts.previewTitle") }}</span>
        </div>
        <div class="result-path">{{ t("tts.previewHint") }}</div>
        <AudioPlayer :vocals-path="dryOutputPath" :bgm-path="bgmPath" />
      </section>
    </div>
  </div>
</template>

<style scoped>
.tts-content {
  flex: 1;
  padding: 0 32px 32px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.tts-card {
  border: 1px solid var(--border);
  background: var(--bg-card);
  border-radius: 14px;
  padding: 16px;
}

.tts-card-disabled {
  opacity: 0.72;
}

.tts-hint {
  color: var(--text-secondary);
  font-size: 13px;
  margin-bottom: 12px;
}

.tts-step-title {
  margin-bottom: 8px;
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 0.4px;
  text-transform: uppercase;
  color: var(--text-secondary);
}

.tts-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}

.tts-label {
  display: block;
  margin-bottom: 6px;
  color: var(--text-secondary);
  font-size: 12px;
}

.tts-textarea {
  width: 100%;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--bg);
  color: var(--text);
  padding: 10px 12px;
  resize: vertical;
  margin-bottom: 12px;
}

.tts-input {
  width: 100%;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--bg);
  color: var(--text);
  padding: 10px 12px;
}

.tts-speed-wrap {
  margin-top: 12px;
}

.tts-speed-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.tts-speed-value {
  font-size: 12px;
  color: var(--text-secondary);
}

.tts-output {
  margin-top: 14px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--bg);
  padding: 12px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  cursor: pointer;
}

.tts-output:hover {
  background: var(--bg-hover);
}

.tts-output.disabled {
  cursor: not-allowed;
}

.tts-output.disabled:hover {
  background: var(--bg);
}

.tts-output-left {
  min-width: 0;
}

.tts-output-path {
  color: var(--text-secondary);
  font-size: 12px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tts-output-path.empty {
  color: var(--text-muted);
}

.tts-output-arrow {
  color: var(--text-muted);
}

.tts-loop-toggle {
  margin-top: 12px;
  min-height: 40px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--bg);
  padding: 0 12px;
  display: inline-flex;
  align-items: center;
  gap: 10px;
}

.tts-loop-label {
  font-size: 13px;
  color: var(--text-secondary);
}

.result-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}

.result-title {
  font-weight: 600;
}

.result-path {
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 10px;
  word-break: break-all;
}

.tts-ref-audio {
  margin-top: 12px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--bg);
  padding: 12px;
}

.tts-ref-actions {
  display: flex;
  gap: 8px;
  margin-top: 4px;
}

.tts-ref-preview {
  margin-top: 10px;
}

.tts-ref-path {
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 6px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tts-ref-player {
  width: 100%;
  height: 32px;
}

.tts-ref-text-input {
  width: 100%;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-card);
  color: var(--text);
  padding: 8px 10px;
  font-size: 13px;
}

.tts-ref-hint {
  margin-top: 4px;
  font-size: 11px;
  color: var(--text-muted);
}

.mt-2 {
  margin-top: 8px;
}

@media (max-width: 900px) {
  .tts-content {
    padding: 0 16px 16px;
  }

  .tts-grid {
    grid-template-columns: 1fr;
  }
}
</style>
