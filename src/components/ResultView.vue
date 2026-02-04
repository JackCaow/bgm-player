<script setup lang="ts">
import { ref, watch, computed, nextTick } from "vue";
import { Icon } from "@iconify/vue";
import { useI18n } from "vue-i18n";
import type { FileItem, TrackInfo } from "@/types";
import MergeSection from "./MergeSection.vue";
import MultiTrackMergeSection from "./MultiTrackMergeSection.vue";
import AudioPlayer from "./AudioPlayer.vue";
import { useWaveform } from "@/composables/useWaveform";

const { t } = useI18n();
const { drawWaveform } = useWaveform();

const props = defineProps<{
  selectedFile: FileItem | null;
  doneCount: number;
  bgmVolume: number;
  vocalsVolume: number;
  isMerging: boolean;
  mergedPath: string | null;
}>();

const emit = defineEmits<{
  clearCompleted: [];
  "update:bgmVolume": [value: number];
  "update:vocalsVolume": [value: number];
  merge: [];
  mergeMultiTrack: [selectedTracks: { path: string; volume: number }[]];
  resetMerge: [];
}>();

const waveformCanvases = ref<Map<string, HTMLCanvasElement>>(new Map());

function setCanvasRef(trackType: string, el: HTMLCanvasElement | null) {
  if (el) {
    waveformCanvases.value.set(trackType, el);
    console.log("[ResultView] Canvas ref set for:", trackType, el);
  }
}

// Get tracks from result with backward compatibility
const tracks = computed<TrackInfo[]>(() => {
  if (!props.selectedFile?.result) return [];

  // New format: use tracks array
  if (props.selectedFile.result.tracks) {
    return props.selectedFile.result.tracks;
  }

  // Legacy format: convert bgmPath and vocalsPath to tracks
  if (props.selectedFile.result.bgmPath && props.selectedFile.result.vocalsPath) {
    return [
      {
        track_type: "vocals",
        path: props.selectedFile.result.vocalsPath,
        name: t("tracks.vocals"),
      },
      {
        track_type: "no_vocals",
        path: props.selectedFile.result.bgmPath,
        name: t("tracks.no_vocals"),
      },
    ];
  }

  return [];
});

// Check if it's a legacy 2-track result
const isLegacyTwoTrack = computed(() => {
  return tracks.value.length === 2 &&
         tracks.value.some(t => t.track_type === "vocals") &&
         tracks.value.some(t => t.track_type === "no_vocals");
});

// Draw waveforms for all tracks when tracks change
watch(
  () => tracks.value,
  async (newTracks) => {
    if (newTracks.length > 0) {
      console.log("[ResultView] Tracks changed, drawing waveforms for:", newTracks.length, "tracks");
      await nextTick();
      // Wait longer for canvas to be rendered
      setTimeout(() => {
        console.log("[ResultView] Canvas map size:", waveformCanvases.value.size);
        for (const track of newTracks) {
          const canvas = waveformCanvases.value.get(track.track_type);
          console.log("[ResultView] Drawing waveform for:", track.track_type, "canvas:", !!canvas, "path:", track.path);
          if (canvas) {
            drawWaveform(track.path, canvas);
          }
        }
      }, 300);
    }
  },
  { deep: true, immediate: true }
);

function getTrackIcon(trackType: string): string {
  const icons: Record<string, string> = {
    vocals: "solar:microphone-bold-duotone",
    drums: "ph:metronome-fill",
    bass: "solar:soundwave-bold-duotone",
    other: "solar:music-notes-bold-duotone",
    guitar: "ph:guitar-fill",
    piano: "ph:piano-keys-fill",
    no_vocals: "solar:music-library-2-bold-duotone",
  };
  return icons[trackType] || "solar:music-note-bold-duotone";
}
</script>

<template>
  <div class="content-panel">
    <div class="panel-header">
      <h2>{{ t("result.title") }}</h2>
      <button v-if="doneCount > 0" class="clear-btn" @click="emit('clearCompleted')">
        {{ t("result.clearCompleted") }}
      </button>
    </div>

    <div v-if="!selectedFile || selectedFile.status !== 'done'" class="empty-content">
      <Icon icon="solar:music-notes-bold-duotone" width="64" class="empty-icon" />
      <span>{{ t("result.emptyHint") }}</span>
    </div>

    <div v-else class="result-detail">
      <div class="result-header">
        <h3>{{ selectedFile.name }}</h3>
        <span v-if="tracks.length > 0" class="track-count">
          {{ tracks.length }} {{ t("tracks.all") }}
        </span>
      </div>

      <!-- Multi-track display -->
      <div v-if="tracks.length > 0" class="tracks-container">
        <div
          v-for="track in tracks"
          :key="track.track_type"
          class="track-card"
        >
          <div class="track-header">
            <Icon :icon="getTrackIcon(track.track_type)" width="20" />
            <span class="track-name">{{ track.name }}</span>
          </div>
          <div class="track-waveform">
            <canvas
              :ref="(el) => setCanvasRef(track.track_type, el as HTMLCanvasElement)"
              width="400"
              height="60"
            ></canvas>
          </div>
        </div>
      </div>

      <!-- Audio Player - use legacy props for 2-track, new props for multi-track -->
      <AudioPlayer
        v-if="isLegacyTwoTrack && selectedFile.result?.bgmPath && selectedFile.result?.vocalsPath"
        :bgm-path="selectedFile.result.bgmPath"
        :vocals-path="selectedFile.result.vocalsPath"
      />
      <AudioPlayer
        v-else-if="tracks.length > 0"
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
</template>

<style scoped>
.result-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 20px;
}

.track-count {
  font-size: 13px;
  color: var(--text-secondary);
  padding: 4px 12px;
  background: var(--bg-card);
  border-radius: 12px;
  border: 1px solid var(--border);
}

.tracks-container {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 16px;
  margin-bottom: 24px;
}

.track-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 16px;
  transition: all 0.2s;
}

.track-card:hover {
  border-color: var(--primary);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
}

.track-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
}

.track-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text);
}

.track-waveform {
  width: 100%;
  height: 60px;
  background: var(--bg);
  border-radius: 8px;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
}

.track-waveform canvas {
  width: 100%;
  height: 100%;
}
</style>