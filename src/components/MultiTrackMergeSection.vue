<script setup lang="ts">
import { ref, computed } from "vue";
import { Icon } from "@iconify/vue";
import { useI18n } from "vue-i18n";
import type { TrackInfo } from "@/types";
import { Checkbox } from "@/components/ui/checkbox";

const { t } = useI18n();

const props = defineProps<{
  tracks: TrackInfo[];
  isMerging: boolean;
  mergedPath: string | null;
}>();

const emit = defineEmits<{
  merge: [selectedTracks: { path: string; volume: number }[]];
  reset: [];
}>();

// Track selection and volume state
const trackStates = ref<Map<string, { selected: boolean; volume: number }>>(new Map());

// Initialize track states
function initTrackStates() {
  props.tracks.forEach(track => {
    if (!trackStates.value.has(track.track_type)) {
      trackStates.value.set(track.track_type, { selected: true, volume: 1.0 });
    }
  });
}
initTrackStates();

const selectedTracks = computed(() => {
  return props.tracks.filter(track => {
    const state = trackStates.value.get(track.track_type);
    return state?.selected;
  });
});

const canMerge = computed(() => selectedTracks.value.length >= 1 && !props.isMerging);

function toggleTrack(trackType: string, checked?: boolean) {
  const state = trackStates.value.get(trackType);
  if (state) {
    state.selected = checked ?? !state.selected;
  }
}

function updateVolume(trackType: string, volume: number) {
  const state = trackStates.value.get(trackType);
  if (state) {
    state.volume = volume;
  }
}

function getVolume(trackType: string): number {
  return trackStates.value.get(trackType)?.volume ?? 1.0;
}

function isSelected(trackType: string): boolean {
  return trackStates.value.get(trackType)?.selected ?? false;
}

function resetVolumes() {
  const newStates = new Map<string, { selected: boolean; volume: number }>();
  props.tracks.forEach(track => {
    newStates.set(track.track_type, { selected: true, volume: 1.0 });
  });
  trackStates.value = newStates;
  emit("reset");
}

function handleMerge() {
  const tracksToMerge = props.tracks
    .filter(track => isSelected(track.track_type))
    .map(track => ({
      path: track.path,
      volume: getVolume(track.track_type),
    }));
  emit("merge", tracksToMerge);
}

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
  <div class="merge-section">
    <div class="merge-header">
      <Icon icon="solar:layers-bold-duotone" width="20" />
      <span>{{ t("merge.title") }}</span>
      <button class="reset-btn" @click="resetVolumes">
        <Icon icon="solar:restart-bold" width="16" />
        {{ t("merge.reset") }}
      </button>
    </div>

    <div class="track-selector">
      <div
        v-for="track in tracks"
        :key="track.track_type"
        class="track-item"
        :class="{ selected: isSelected(track.track_type) }"
      >
        <label class="track-checkbox">
          <Checkbox
            :model-value="isSelected(track.track_type)"
            @update:model-value="(checked: boolean | 'indeterminate') => toggleTrack(track.track_type, checked === true)"
          />
          <Icon :icon="getTrackIcon(track.track_type)" width="16" />
          <span>{{ track.name }}</span>
        </label>
        <div class="track-volume">
          <input
            type="range"
            :value="getVolume(track.track_type)"
            @input="updateVolume(track.track_type, Number(($event.target as HTMLInputElement).value))"
            min="0"
            max="2"
            step="0.05"
            class="volume-slider-small"
            :disabled="!isSelected(track.track_type)"
          />
          <span class="volume-value">{{ Math.round(getVolume(track.track_type) * 100) }}%</span>
        </div>
      </div>
    </div>

    <button class="merge-btn" :disabled="!canMerge" @click="handleMerge">
      <template v-if="!isMerging">
        <Icon icon="solar:layers-bold" width="18" />
        {{ t("merge.mergeButton") }}
        <span v-if="selectedTracks.length > 0" class="merge-count">
          ({{ selectedTracks.length }} {{ t("merge.tracksCount") }})
        </span>
      </template>
      <template v-else>
        <div class="spinner"></div>
        {{ t("merge.merging") }}
      </template>
    </button>

    <div v-if="mergedPath" class="merge-result">
      <div class="merge-result-header">
        <Icon icon="solar:check-circle-bold" width="18" class="icon-success" />
        <span>{{ t("merge.success") }}</span>
      </div>
      <audio controls :src="'asset://localhost/' + mergedPath" class="merged-audio"></audio>
      <div class="merge-path">{{ mergedPath }}</div>
    </div>
  </div>
</template>

<style scoped>
.merge-section {
  margin-top: 24px;
  padding: 20px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
}

.merge-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 16px;
  font-weight: 600;
  color: var(--text);
}

.reset-btn {
  margin-left: auto;
  padding: 6px 12px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary);
  font-size: 12px;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 4px;
  transition: all 0.2s;
}

.reset-btn:hover {
  background: var(--bg-hover);
  color: var(--text);
}

.track-selector {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 16px;
}

.track-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 8px;
  transition: all 0.2s;
}

.track-item.selected {
  border-color: var(--primary);
  background: rgba(29, 185, 84, 0.05);
}

.track-checkbox {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  font-size: 13px;
  font-weight: 500;
  color: var(--text);
}

.track-volume {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 160px;
}

.volume-slider-small {
  flex: 1;
  height: 4px;
  border-radius: 2px;
  -webkit-appearance: none;
  appearance: none;
  background: var(--border);
  cursor: pointer;
}

.volume-slider-small:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.volume-slider-small::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--primary);
  cursor: pointer;
}

.volume-value {
  font-size: 11px;
  color: var(--text-secondary);
  min-width: 36px;
  text-align: right;
}

.merge-btn {
  width: 100%;
  padding: 12px 20px;
  border: none;
  border-radius: 8px;
  background: var(--primary);
  color: #000;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  transition: all 0.2s;
}

.merge-btn:hover:not(:disabled) {
  filter: brightness(1.1);
}

.merge-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.merge-count {
  font-weight: normal;
  opacity: 0.8;
}

.spinner {
  width: 18px;
  height: 18px;
  border: 2px solid transparent;
  border-top-color: currentColor;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.merge-result {
  margin-top: 16px;
  padding: 12px;
  background: rgba(29, 185, 84, 0.1);
  border-radius: 8px;
}

.merge-result-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
  font-weight: 500;
  color: var(--success);
}

.merged-audio {
  width: 100%;
  height: 36px;
  margin-bottom: 8px;
}

.merge-path {
  font-size: 11px;
  color: var(--text-secondary);
  word-break: break-all;
}
</style>
