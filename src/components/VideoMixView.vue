<script setup lang="ts">
import { computed } from "vue";
import { Icon } from "@iconify/vue";
import { useI18n } from "vue-i18n";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import Slider from "@/components/ui/Slider.vue";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { VideoMixMode } from "@/composables/useVideoMix";
import { formatPath } from "@/utils/format";

const { t } = useI18n();

const props = defineProps<{
  videoPath: string;
  bgmPath: string;
  outputDir: string;
  mode: VideoMixMode;
  bgmVolume: number;
  videoVolume: number;
  bgmOffsetSec: number;
  loopBgm: boolean;
  fadeInSec: number;
  fadeOutSec: number;
  isMixing: boolean;
  progress: number;
  status: string;
  outputPath: string | null;
  canStart: boolean;
}>();

const emit = defineEmits<{
  selectVideo: [];
  selectBgm: [];
  selectOutputDir: [];
  start: [];
  clear: [];
  "update:mode": [value: VideoMixMode];
  "update:bgmVolume": [value: number];
  "update:videoVolume": [value: number];
  "update:bgmOffsetSec": [value: number];
  "update:loopBgm": [value: boolean];
  "update:fadeInSec": [value: number];
  "update:fadeOutSec": [value: number];
}>();

const progressPct = computed(() => {
  const p = Number(props.progress);
  if (!Number.isFinite(p)) return 0;
  return Math.min(100, Math.max(0, p));
});

const modeOptions = computed<{ value: VideoMixMode; label: string }[]>(() => [
  { value: "mix", label: t("videoMix.modeMix") },
  { value: "bgmOnly", label: t("videoMix.modeBgmOnly") },
]);

function updateNumber(
  event: Event,
  updater: (value: number) => void,
) {
  const input = event.target as HTMLInputElement;
  const value = Number(input.value);
  updater(Number.isFinite(value) ? Math.max(0, value) : 0);
}
</script>

<template>
  <div class="content-panel">
    <div class="panel-header">
      <h2>{{ t("videoMix.title") }}</h2>
      <Button
        v-if="videoPath || bgmPath"
        variant="outline"
        size="sm"
        @click="emit('clear')"
      >
        {{ t("videoMix.clear") }}
      </Button>
    </div>

    <div class="video-mix-content">
      <section class="vm-card">
        <div class="vm-card-title">{{ t("videoMix.title") }}</div>
        <div class="vm-source-list">
          <Button variant="outline" class="vm-source-item" @click="emit('selectVideo')">
            <div class="vm-source-icon">
              <Icon icon="solar:videocamera-record-bold-duotone" width="20" />
            </div>
            <div class="vm-source-meta">
              <div class="vm-source-label">{{ t("videoMix.video") }}</div>
              <div class="vm-source-value" :class="{ muted: !videoPath }">
                {{ videoPath ? formatPath(videoPath) : t("videoMix.selectVideo") }}
              </div>
            </div>
            <Icon icon="solar:alt-arrow-right-linear" width="16" class="vm-source-arrow" />
          </Button>

          <Button variant="outline" class="vm-source-item" @click="emit('selectBgm')">
            <div class="vm-source-icon">
              <Icon icon="solar:music-note-2-bold-duotone" width="20" />
            </div>
            <div class="vm-source-meta">
              <div class="vm-source-label">{{ t("videoMix.bgm") }}</div>
              <div class="vm-source-value" :class="{ muted: !bgmPath }">
                {{ bgmPath ? formatPath(bgmPath) : t("videoMix.selectBgm") }}
              </div>
            </div>
            <Icon icon="solar:alt-arrow-right-linear" width="16" class="vm-source-arrow" />
          </Button>

          <Button variant="outline" class="vm-source-item" @click="emit('selectOutputDir')">
            <div class="vm-source-icon">
              <Icon icon="solar:folder-open-bold-duotone" width="20" />
            </div>
            <div class="vm-source-meta">
              <div class="vm-source-label">{{ t("videoMix.outputDir") }}</div>
              <div class="vm-source-value" :class="{ muted: !outputDir }">
                {{ outputDir ? formatPath(outputDir) : t("videoMix.outputDirOptional") }}
              </div>
            </div>
            <Icon icon="solar:alt-arrow-right-linear" width="16" class="vm-source-arrow" />
          </Button>
        </div>
      </section>

      <section class="vm-card">
        <div class="mb-3 flex items-center gap-2 text-sm font-semibold text-foreground">
          <Icon icon="solar:tuning-2-bold-duotone" width="18" />
          {{ t("videoMix.mode") }}
        </div>

        <Select :model-value="mode" @update:modelValue="emit('update:mode', $event as VideoMixMode)">
          <SelectTrigger class="mb-4 w-full">
            <SelectValue :placeholder="t('videoMix.mode')" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem v-for="option in modeOptions" :key="option.value" :value="option.value">
              {{ option.label }}
            </SelectItem>
          </SelectContent>
        </Select>

        <div class="grid grid-cols-1 gap-3 lg:grid-cols-2">
          <div class="space-y-2">
            <div class="flex items-center justify-between text-xs text-muted-foreground">
              <span>{{ t("videoMix.bgmVolume") }}</span>
              <span>{{ Math.round(bgmVolume * 100) }}%</span>
            </div>
            <Slider
              :model-value="bgmVolume"
              :min="0"
              :max="2"
              :step="0.05"
              @update:model-value="emit('update:bgmVolume', $event)"
            />
          </div>

          <div class="space-y-2" :class="mode !== 'mix' ? 'opacity-60' : ''">
            <div class="flex items-center justify-between text-xs text-muted-foreground">
              <span>{{ t("videoMix.videoVolume") }}</span>
              <span>{{ Math.round(videoVolume * 100) }}%</span>
            </div>
            <Slider
              :model-value="videoVolume"
              :disabled="mode !== 'mix'"
              :min="0"
              :max="2"
              :step="0.05"
              @update:model-value="emit('update:videoVolume', $event)"
            />
          </div>

          <div class="space-y-2">
            <div class="text-xs text-muted-foreground">{{ t("videoMix.offset") }}</div>
            <div class="flex items-center gap-2">
              <input
                type="number"
                min="0"
                step="0.1"
                :value="bgmOffsetSec"
                class="h-10 w-full rounded-md border border-input bg-background px-3 text-sm text-foreground outline-none ring-offset-background focus-visible:ring-2 focus-visible:ring-ring"
                @input="updateNumber($event, (v) => emit('update:bgmOffsetSec', v))"
              />
              <span class="text-xs text-muted-foreground">s</span>
            </div>
          </div>

          <div class="space-y-2">
            <div class="text-xs text-muted-foreground">{{ t("videoMix.loop") }}</div>
            <label class="inline-flex h-10 cursor-pointer items-center gap-2 rounded-md border border-input bg-background px-3 text-sm text-foreground">
              <Checkbox
                :model-value="loopBgm"
                @update:model-value="emit('update:loopBgm', Boolean($event))"
              />
              <span>{{ loopBgm ? t("videoMix.on") : t("videoMix.off") }}</span>
            </label>
          </div>

          <div class="space-y-2">
            <div class="text-xs text-muted-foreground">{{ t("videoMix.fadeIn") }}</div>
            <div class="flex items-center gap-2">
              <input
                type="number"
                min="0"
                step="0.1"
                :value="fadeInSec"
                class="h-10 w-full rounded-md border border-input bg-background px-3 text-sm text-foreground outline-none ring-offset-background focus-visible:ring-2 focus-visible:ring-ring"
                @input="updateNumber($event, (v) => emit('update:fadeInSec', v))"
              />
              <span class="text-xs text-muted-foreground">s</span>
            </div>
          </div>

          <div class="space-y-2">
            <div class="text-xs text-muted-foreground">{{ t("videoMix.fadeOut") }}</div>
            <div class="flex items-center gap-2">
              <input
                type="number"
                min="0"
                step="0.1"
                :value="fadeOutSec"
                class="h-10 w-full rounded-md border border-input bg-background px-3 text-sm text-foreground outline-none ring-offset-background focus-visible:ring-2 focus-visible:ring-ring"
                @input="updateNumber($event, (v) => emit('update:fadeOutSec', v))"
              />
              <span class="text-xs text-muted-foreground">s</span>
            </div>
          </div>
        </div>
      </section>

      <Button class="h-11 w-full rounded-xl" :disabled="!canStart" @click="emit('start')">
        <template v-if="!isMixing">
          <Icon icon="solar:music-notes-bold" width="20" />
          {{ t("videoMix.start") }}
        </template>
        <template v-else>
          <Icon icon="solar:refresh-bold" width="20" class="animate-spin" />
          {{ t("videoMix.mixing") }}
        </template>
      </Button>

      <div v-if="isMixing" class="vm-card">
        <div class="mb-2 flex items-center justify-between gap-3">
          <span class="truncate text-sm text-foreground">{{ status || t("videoMix.processing") }}</span>
          <span class="text-xs text-muted-foreground">{{ Math.round(progressPct) }}%</span>
        </div>
        <div class="h-2 overflow-hidden rounded-full bg-muted">
          <div class="h-full rounded-full bg-primary transition-all duration-200" :style="{ width: progressPct + '%' }"></div>
        </div>
      </div>

      <div v-if="outputPath" class="vm-card">
        <div class="mb-3 flex items-center gap-2">
          <Icon icon="solar:check-circle-bold" width="22" class="text-primary" />
          <div class="min-w-0">
            <div class="text-sm font-semibold text-foreground">{{ t("videoMix.success") }}</div>
            <div class="truncate text-xs text-muted-foreground">{{ outputPath }}</div>
          </div>
        </div>
        <video controls :src="'asset://localhost/' + outputPath" class="w-full rounded-xl bg-black"></video>
      </div>
    </div>
  </div>
</template>

<style scoped>
.video-mix-content {
  flex: 1;
  padding: 0 32px 32px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.vm-card {
  border: 1px solid var(--border);
  background: var(--bg-card);
  border-radius: 14px;
  padding: 18px;
}

.vm-card-title {
  margin-bottom: 12px;
  font-size: 13px;
  font-weight: 700;
  letter-spacing: 0.2px;
  text-transform: uppercase;
  color: var(--text-secondary);
}

.vm-source-list {
  display: grid;
  grid-template-columns: 1fr;
  gap: 10px;
}

.vm-source-item {
  height: auto;
  width: 100%;
  justify-content: flex-start;
  gap: 12px;
  padding: 12px 14px;
  border-radius: 12px;
  border: 1px solid var(--border);
  background: var(--bg);
  text-align: left;
}

.vm-source-item:hover {
  background: var(--bg-hover);
}

.vm-source-icon {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: color-mix(in srgb, var(--primary) 14%, var(--bg));
  color: var(--primary);
  flex-shrink: 0;
}

.vm-source-meta {
  min-width: 0;
  flex: 1;
}

.vm-source-label {
  font-size: 13px;
  font-weight: 700;
  color: var(--text);
}

.vm-source-value {
  margin-top: 2px;
  font-size: 12px;
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.vm-source-value.muted {
  color: var(--text-muted);
}

.vm-source-arrow {
  flex-shrink: 0;
  color: var(--text-muted);
}

.video-mix-content::-webkit-scrollbar {
  width: 8px;
}

.video-mix-content::-webkit-scrollbar-track {
  background: transparent;
}

.video-mix-content::-webkit-scrollbar-thumb {
  background: var(--text-muted);
  border-radius: 4px;
}

.video-mix-content::-webkit-scrollbar-thumb:hover {
  background: var(--text-secondary);
}

@media (max-width: 768px) {
  .video-mix-content {
    padding: 0 16px 16px;
    gap: 12px;
  }

  .vm-card {
    padding: 14px;
    border-radius: 12px;
  }
}
</style>
