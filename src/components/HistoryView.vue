<script setup lang="ts">
import { ref, watch, computed, nextTick } from "vue";
import { Icon } from "@iconify/vue";
import { useI18n } from "vue-i18n";
import type { HistoryItem, TrackInfo } from "@/types";
import { formatDate, formatFullDate, formatDuration } from "@/utils/format";
import MergeSection from "./MergeSection.vue";
import MultiTrackMergeSection from "./MultiTrackMergeSection.vue";
import AudioPlayer from "./AudioPlayer.vue";
import { useWaveform } from "@/composables/useWaveform";

const { t, locale } = useI18n();
const { drawWaveform } = useWaveform();

const props = defineProps<{
  history: HistoryItem[];
  historyCount: number;
  selectedHistoryId: string | null;
  selectedHistory: HistoryItem | null;
  bgmVolume: number;
  vocalsVolume: number;
  isMerging: boolean;
  mergedPath: string | null;
}>();

const emit = defineEmits<{
  clearHistory: [];
  selectHistoryItem: [id: string];
  deleteHistoryItem: [id: string];
  "update:bgmVolume": [value: number];
  "update:vocalsVolume": [value: number];
  merge: [];
  mergeMultiTrack: [selectedTracks: { path: string; volume: number }[]];
  resetMerge: [];
}>();

const historyWaveformCanvas = ref<HTMLCanvasElement | null>(null);

// Get tracks from selected history
const tracks = computed<TrackInfo[]>(() => {
  if (!props.selectedHistory) return [];
  return props.selectedHistory.tracks || [];
});

// Check if it's legacy 2-track mode
const isLegacyTwoTrack = computed(() => {
  return tracks.value.length === 2 &&
         tracks.value.some(t => t.track_type === "vocals") &&
         tracks.value.some(t => t.track_type === "no_vocals");
});

function getTrackIcon(trackType: string): string {
  const icons: Record<string, string> = {
    vocals: "solar:microphone-3-bold-duotone",
    drums: "solar:music-note-2-bold-duotone",
    bass: "solar:soundwave-bold-duotone",
    other: "solar:music-notes-bold-duotone",
    guitar: "solar:guitar-bold-duotone",
    piano: "solar:piano-bold-duotone",
    no_vocals: "solar:music-note-2-bold-duotone",
  };
  return icons[trackType] || "solar:music-note-bold-duotone";
}

watch(
  () => tracks.value[0]?.path,
  async (newPath) => {
    if (newPath) {
      await nextTick();
      setTimeout(() => drawWaveform(newPath, historyWaveformCanvas.value), 300);
    }
  },
  { immediate: true }
);
</script>

<template>
  <div class="content-panel">
    <div class="panel-header">
      <h2>{{ t("history.title") }}</h2>
      <button v-if="historyCount > 0" class="clear-btn" @click="emit('clearHistory')">
        {{ t("history.clearAll") }}
      </button>
    </div>

    <div v-if="historyCount === 0" class="empty-content">
      <Icon icon="solar:history-bold-duotone" width="64" class="empty-icon" />
      <span>{{ t("history.emptyHint") }}</span>
    </div>

    <div v-else class="history-layout">
      <!-- History List -->
      <div class="history-list">
        <div
          v-for="item in history"
          :key="item.id"
          class="history-item"
          :class="{ active: selectedHistoryId === item.id }"
          @click="emit('selectHistoryItem', item.id)"
        >
          <div class="history-item-icon">
            <Icon icon="solar:music-note-2-bold-duotone" width="20" />
          </div>
          <div class="history-item-info">
            <span class="history-item-name">{{ item.name }}</span>
            <span class="history-item-meta">
              {{ item.model }} · {{ formatDate(item.processedAt, locale) }}
            </span>
          </div>
          <button class="history-item-delete" @click.stop="emit('deleteHistoryItem', item.id)">
            <Icon icon="solar:trash-bin-minimalistic-bold" width="16" />
          </button>
        </div>
      </div>

      <!-- History Detail -->
      <div v-if="selectedHistory" class="history-detail">
        <div class="result-header">
          <h3>{{ selectedHistory.name }}</h3>
        </div>

        <!-- History Info Cards -->
        <div class="history-info-grid">
          <div class="info-card">
            <div class="info-icon">
              <Icon icon="solar:calendar-bold-duotone" width="20" />
            </div>
            <div class="info-content">
              <span class="info-label">{{ t("history.processedAt") }}</span>
              <span class="info-value">
                {{ formatFullDate(selectedHistory.processedAt, locale) }}
              </span>
            </div>
          </div>
          <div class="info-card">
            <div class="info-icon">
              <Icon icon="solar:cpu-bolt-bold-duotone" width="20" />
            </div>
            <div class="info-content">
              <span class="info-label">{{ t("history.model") }}</span>
              <span class="info-value">{{ selectedHistory.model }}</span>
            </div>
          </div>
          <div v-if="selectedHistory.processingTime" class="info-card">
            <div class="info-icon">
              <Icon icon="solar:stopwatch-bold-duotone" width="20" />
            </div>
            <div class="info-content">
              <span class="info-label">{{ t("history.processingTime") }}</span>
              <span class="info-value">
                {{ formatDuration(selectedHistory.processingTime, locale) }}
              </span>
            </div>
          </div>
        </div>

        <!-- Source File -->
        <div class="history-file-section">
          <div class="file-section-header">
            <Icon icon="solar:file-bold-duotone" width="18" />
            <span>{{ t("history.sourceFile") }}</span>
          </div>
          <div class="file-path-box">
            <span class="file-path-text">{{ selectedHistory.sourcePath }}</span>
          </div>
        </div>

        <!-- Output Files -->
        <div class="history-file-section">
          <div class="file-section-header">
            <Icon icon="solar:folder-check-bold-duotone" width="18" />
            <span>{{ t("history.outputFiles") }}</span>
          </div>
          <div class="output-files-list">
            <div v-for="track in tracks" :key="track.track_type" class="output-file-item">
              <Icon :icon="getTrackIcon(track.track_type)" width="16" :class="'icon-' + track.track_type" />
              <span class="output-file-path">{{ track.path }}</span>
            </div>
          </div>
        </div>

        <div class="waveform-section">
          <canvas ref="historyWaveformCanvas" class="waveform" width="400" height="80"></canvas>
        </div>

        <!-- Audio Player -->
        <AudioPlayer
          v-if="tracks.length > 0"
          :tracks="tracks"
        />

        <MergeSection
          v-if="isLegacyTwoTrack"
          :bgm-volume="bgmVolume"
          :vocals-volume="vocalsVolume"
          :is-merging="isMerging"
          :merged-path="mergedPath"
          @update:bgm-volume="emit('update:bgmVolume', $event)"
          @update:vocals-volume="emit('update:vocalsVolume', $event)"
          @merge="emit('merge')"
          @reset="emit('resetMerge')"
        />
        <MultiTrackMergeSection
          v-else-if="tracks.length > 0"
          :tracks="tracks"
          :is-merging="isMerging"
          :merged-path="mergedPath"
          @merge="emit('mergeMultiTrack', $event)"
          @reset="emit('resetMerge')"
        />
      </div>
    </div>
  </div>
</template>
