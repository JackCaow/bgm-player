<script setup lang="ts">
import { computed } from "vue";
import { Icon } from "@iconify/vue";
import { useI18n } from "vue-i18n";
import { useGlobalPlayer } from "@/composables/useGlobalPlayer";
import type { TrackInfo, TrackType } from "@/types";
import Slider from "./ui/Slider.vue";

const { t } = useI18n();

const props = defineProps<{
  bgmPath?: string;
  vocalsPath?: string;
  tracks?: TrackInfo[];
}>();

// Determine if we're in legacy mode or multi-track mode
const isLegacyMode = computed(() => !!props.bgmPath && !!props.vocalsPath);

// Convert legacy props to tracks format
const effectiveTracks = computed<TrackInfo[]>(() => {
  if (props.tracks) {
    return props.tracks;
  }
  if (props.bgmPath && props.vocalsPath) {
    return [
      { track_type: "vocals", path: props.vocalsPath, name: t("tracks.vocals") },
      { track_type: "no_vocals", path: props.bgmPath, name: t("tracks.no_vocals") },
    ];
  }
  return [];
});

const {
  isPlaying,
  progress,
  formattedCurrentTime,
  formattedDuration,
  activeTracks,
  currentTracks,
  loadTracks,
  play,
  togglePlayPause,
  seekToPercent,
  setTrackVolume,
  getTrackVolume,
  toggleTrack,
} = useGlobalPlayer();

// Check if current tracks match the displayed tracks
const isCurrentContent = computed(() => {
  if (currentTracks.value.length !== effectiveTracks.value.length) return false;
  return effectiveTracks.value.every((t, i) => currentTracks.value[i]?.path === t.path);
});

// Handle play button click - only load if different content
async function handlePlayClick() {
  if (!isCurrentContent.value) {
    // Load new tracks
    try {
      await loadTracks(effectiveTracks.value);
      play();
    } catch (error) {
      console.error("Failed to load audio:", error);
    }
  } else {
    // Same content, just toggle play/pause
    togglePlayPause();
  }
}

function handleProgressChange(value: number) {
  if (isCurrentContent.value) {
    seekToPercent(value);
  }
}

function handleTrackVolumeChange(trackType: TrackType, value: number) {
  if (isCurrentContent.value) {
    setTrackVolume(trackType, value / 100);
  }
}

function getTrackVolumePercent(trackType: TrackType): number {
  if (isCurrentContent.value) {
    return getTrackVolume(trackType) * 100;
  }
  return 100;
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

function isTrackActive(trackType: TrackType): boolean {
  if (isCurrentContent.value) {
    return activeTracks.value.has(trackType);
  }
  return true; // Show all as active when not current content
}

function handleToggleTrack(trackType: TrackType) {
  if (isCurrentContent.value) {
    toggleTrack(trackType);
  }
}

// Computed values for display
const displayProgress = computed(() => isCurrentContent.value ? progress.value : 0);
const displayCurrentTime = computed(() => isCurrentContent.value ? formattedCurrentTime.value : "0:00");
const displayDuration = computed(() => isCurrentContent.value ? formattedDuration.value : "0:00");
const displayIsPlaying = computed(() => isCurrentContent.value && isPlaying.value);
</script>

<template>
  <div class="audio-player">
    <!-- Progress Bar -->
    <div class="progress-section">
      <div class="time-display">{{ displayCurrentTime }}</div>
      <Slider
        :model-value="displayProgress"
        :min="0"
        :max="100"
        :step="0.1"
        @update:model-value="handleProgressChange"
      />
      <div class="time-display">{{ displayDuration }}</div>
    </div>

    <!-- Playback Controls -->
    <div class="controls-section">
      <div class="main-controls">
        <!-- Track Selection Buttons (for multi-track) -->
        <div v-if="!isLegacyMode" class="track-buttons multi-track">
          <button
            v-for="track in effectiveTracks"
            :key="track.track_type"
            class="track-btn"
            :class="{ active: isTrackActive(track.track_type as TrackType) }"
            @click="handleToggleTrack(track.track_type as TrackType)"
            :title="track.name"
          >
            <Icon :icon="getTrackIcon(track.track_type)" width="18" />
            <span>{{ track.name }}</span>
          </button>
        </div>

        <!-- Legacy 2-track buttons -->
        <div v-else class="track-buttons">
          <button
            class="track-btn"
            :class="{ active: isTrackActive('vocals') }"
            @click="handleToggleTrack('vocals')"
            :title="t('player.playVocals')"
          >
            <Icon icon="solar:microphone-bold-duotone" width="18" />
            <span>{{ t('tracks.vocals') }}</span>
          </button>

          <button
            class="track-btn"
            :class="{ active: isTrackActive('no_vocals') }"
            @click="handleToggleTrack('no_vocals')"
            :title="t('player.playBgm')"
          >
            <Icon icon="solar:music-library-2-bold-duotone" width="18" />
            <span>BGM</span>
          </button>
        </div>

        <!-- Play/Pause Button -->
        <button class="play-btn" @click="handlePlayClick" :title="displayIsPlaying ? t('player.pause') : t('player.play')">
          <Icon v-if="!displayIsPlaying" icon="solar:play-bold" width="28" />
          <Icon v-else icon="solar:pause-bold" width="28" />
        </button>
      </div>

      <!-- Volume Controls -->
      <div class="volume-section" :class="{ 'multi-track-volume': effectiveTracks.length > 2 }">
        <div
          v-for="track in effectiveTracks"
          :key="track.track_type"
          class="volume-control"
        >
          <div class="volume-label">
            <Icon :icon="getTrackIcon(track.track_type)" width="16" />
            <span>{{ track.name }}</span>
          </div>
          <div class="volume-slider-wrapper">
            <Slider
              :model-value="getTrackVolumePercent(track.track_type as TrackType)"
              :min="0"
              :max="200"
              :step="1"
              @update:model-value="(v) => handleTrackVolumeChange(track.track_type as TrackType, v)"
            />
            <span class="volume-value">{{ Math.round(getTrackVolumePercent(track.track_type as TrackType)) }}%</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.audio-player {
  background: linear-gradient(135deg, rgba(40, 40, 40, 0.6) 0%, rgba(30, 30, 30, 0.8) 100%);
  backdrop-filter: blur(20px);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 20px;
  padding: 28px 32px;
  margin-top: 20px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
  overflow: hidden;
}

/* Progress Section */
.progress-section {
  display: flex;
  align-items: center;
  gap: 20px;
  margin-bottom: 28px;
  min-width: 0;
}

.time-display {
  font-size: 14px;
  color: rgba(255, 255, 255, 0.7);
  font-variant-numeric: tabular-nums;
  min-width: 48px;
  text-align: center;
  font-weight: 600;
  letter-spacing: 0.3px;
  flex-shrink: 0;
}

.progress-bar {
  flex: 1;
  cursor: pointer;
  padding: 10px 0;
  position: relative;
}

.progress-track {
  position: relative;
  height: 5px;
  background: rgba(255, 255, 255, 0.15);
  border-radius: 4px;
  overflow: hidden;
  transition: height 0.2s;
}

.progress-bar:hover .progress-track {
  height: 6px;
}

.progress-fill {
  position: absolute;
  top: 0;
  left: 0;
  height: 100%;
  background: linear-gradient(90deg, #1DB954 0%, #1ed760 100%);
  border-radius: 4px;
  transition: width 0.1s linear;
  position: relative;
  overflow: hidden;
}

.progress-fill::after {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: linear-gradient(90deg,
    transparent 0%,
    rgba(255, 255, 255, 0.3) 50%,
    transparent 100%);
  transform: translateX(-100%);
  animation: shine 3s infinite;
}

@keyframes shine {
  0% { transform: translateX(-100%); }
  50%, 100% { transform: translateX(200%); }
}

.progress-thumb {
  position: absolute;
  top: 50%;
  transform: translate(-50%, -50%) scale(0);
  width: 14px;
  height: 14px;
  background: #ffffff;
  border-radius: 50%;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4);
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  cursor: grab;
  z-index: 2;
}

.progress-thumb::before {
  content: '';
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  width: 24px;
  height: 24px;
  background: rgba(29, 185, 84, 0.2);
  border-radius: 50%;
  transition: all 0.2s;
}

.progress-thumb:active {
  cursor: grabbing;
}

.progress-bar:hover .progress-thumb {
  transform: translate(-50%, -50%) scale(1);
}

.progress-bar:active .progress-thumb,
.progress-thumb:hover {
  transform: translate(-50%, -50%) scale(1.3);
  box-shadow: 0 3px 12px rgba(0, 0, 0, 0.5);
}

.progress-thumb:hover::before {
  width: 32px;
  height: 32px;
  background: rgba(29, 185, 84, 0.3);
}

/* Controls Section */
.controls-section {
  display: flex;
  flex-direction: column;
  gap: 24px;
  min-width: 0;
}

.main-controls {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 20px;
  min-width: 0;
  flex-wrap: wrap;
}

/* Track Buttons */
.track-buttons {
  display: flex;
  gap: 10px;
  flex: 1;
  min-width: 0;
  flex-wrap: wrap;
}

.track-btn {
  flex: 1 1 0;
  min-width: 100px;
  max-width: 200px;
  padding: 12px 20px;
  border: 2px solid rgba(255, 255, 255, 0.1);
  border-radius: 28px;
  background: rgba(255, 255, 255, 0.05);
  backdrop-filter: blur(10px);
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  font-size: 14px;
  font-weight: 700;
  min-height: 48px;
  letter-spacing: 0.3px;
  position: relative;
  overflow: hidden;
}

.track-btn span {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.track-btn::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: linear-gradient(135deg, rgba(29, 185, 84, 0.1) 0%, rgba(30, 215, 96, 0.1) 100%);
  opacity: 0;
  transition: opacity 0.3s;
}

.track-btn:hover {
  border-color: rgba(29, 185, 84, 0.5);
  color: var(--text);
  transform: translateY(-2px);
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.25);
}

.track-btn:hover::before {
  opacity: 1;
}

.track-btn.active {
  background: linear-gradient(135deg, #1DB954 0%, #1ed760 100%);
  color: white;
  border-color: transparent;
  box-shadow: 0 8px 24px rgba(29, 185, 84, 0.4), 0 0 40px rgba(29, 185, 84, 0.2);
  transform: translateY(-2px);
}

.track-btn.active::before {
  opacity: 0;
}

.track-btn.both-btn.active {
  background: linear-gradient(135deg, #1DB954 0%, #1ed760 50%, #1DB954 100%);
  animation: shimmer 3s ease-in-out infinite;
}

@keyframes shimmer {
  0%, 100% { background-position: 0% 50%; }
  50% { background-position: 100% 50%; }
}

/* Play Button */
.play-btn {
  width: 68px;
  height: 68px;
  border: none;
  border-radius: 50%;
  background: linear-gradient(135deg, #1DB954 0%, #1ed760 100%);
  color: white;
  cursor: pointer;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  box-shadow: 0 8px 28px rgba(29, 185, 84, 0.4), 0 0 0 0 rgba(29, 185, 84, 0.4);
  position: relative;
  overflow: hidden;
}

.play-btn::before {
  content: '';
  position: absolute;
  top: 50%;
  left: 50%;
  width: 100%;
  height: 100%;
  background: radial-gradient(circle, rgba(255, 255, 255, 0.3) 0%, transparent 70%);
  transform: translate(-50%, -50%) scale(0);
  transition: transform 0.6s;
}

.play-btn:hover {
  transform: scale(1.1);
  box-shadow: 0 12px 36px rgba(29, 185, 84, 0.5), 0 0 0 8px rgba(29, 185, 84, 0.1);
}

.play-btn:hover::before {
  transform: translate(-50%, -50%) scale(1);
}

.play-btn:active {
  transform: scale(1.05);
}

/* Volume Section */
.volume-section {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 24px;
  padding-top: 24px;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  min-width: 0;
}

.volume-section.multi-track-volume {
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 16px;
}

.track-buttons.multi-track {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(90px, 1fr));
  gap: 8px;
}

.track-buttons.multi-track .track-btn {
  font-size: 12px;
  padding: 10px 12px;
  min-height: 42px;
  min-width: 90px;
}

.volume-control {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-width: 0;
}

.volume-label {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--text-secondary);
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 1px;
  opacity: 0.9;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.volume-slider-wrapper {
  display: flex;
  align-items: center;
  gap: 14px;
  min-width: 0;
}

.volume-slider {
  flex: 1;
  height: 8px;
  border-radius: 4px;
  outline: none;
  background: rgba(255, 255, 255, 0.1);
  -webkit-appearance: none;
  appearance: none;
  cursor: pointer;
  position: relative;
  box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.2);
}

.volume-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: #ffffff;
  cursor: pointer;
  border: 3px solid #1DB954;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3), 0 0 0 4px rgba(29, 185, 84, 0.2);
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  margin-top: -5px; /* 修复垂直居中 */
}

.volume-slider::-moz-range-thumb {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: #ffffff;
  cursor: pointer;
  border: 3px solid #1DB954;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3), 0 0 0 4px rgba(29, 185, 84, 0.2);
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

.volume-slider:hover::-webkit-slider-thumb {
  transform: scale(1.2);
  box-shadow: 0 3px 12px rgba(0, 0, 0, 0.4), 0 0 0 6px rgba(29, 185, 84, 0.3);
}

.volume-slider:hover::-moz-range-thumb {
  transform: scale(1.2);
  box-shadow: 0 3px 12px rgba(0, 0, 0, 0.4), 0 0 0 6px rgba(29, 185, 84, 0.3);
}

.volume-slider::-webkit-slider-runnable-track {
  height: 8px;
  border-radius: 4px;
  background: linear-gradient(
    to right,
    #1DB954 0%,
    #1DB954 var(--slider-percent, 100%),
    rgba(255, 255, 255, 0.1) var(--slider-percent, 100%),
    rgba(255, 255, 255, 0.1) 100%
  );
}

.volume-value {
  font-size: 14px;
  color: var(--text);
  min-width: 50px;
  text-align: right;
  font-variant-numeric: tabular-nums;
  font-weight: 700;
  letter-spacing: 0.3px;
}

/* Responsive */
@media (max-width: 768px) {
  .audio-player {
    padding: 20px 24px;
  }

  .progress-section {
    gap: 12px;
  }

  .time-display {
    min-width: 40px;
    font-size: 12px;
  }

  .volume-section {
    grid-template-columns: 1fr;
    gap: 20px;
  }

  .main-controls {
    flex-direction: column;
    gap: 16px;
  }

  .track-buttons {
    width: 100%;
    flex-wrap: wrap;
  }

  .track-btn {
    min-width: 70px;
    padding: 10px 16px;
    font-size: 13px;
  }

  .play-btn {
    width: 60px;
    height: 60px;
  }
}

@media (max-width: 480px) {
  .audio-player {
    padding: 16px 20px;
  }

  .progress-section {
    gap: 8px;
  }

  .time-display {
    min-width: 36px;
    font-size: 11px;
  }

  .track-btn {
    min-width: 60px;
    padding: 8px 12px;
    font-size: 12px;
    gap: 6px;
  }

  .track-btn span {
    display: none;
  }

  .play-btn {
    width: 56px;
    height: 56px;
  }

  .volume-label {
    font-size: 10px;
  }

  .volume-value {
    font-size: 12px;
    min-width: 42px;
  }
}

/* Light theme adjustments */
[data-theme="light"] .audio-player {
  background: linear-gradient(135deg, rgba(255, 255, 255, 0.9) 0%, rgba(245, 245, 245, 0.95) 100%);
  border: 1px solid rgba(0, 0, 0, 0.08);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.08);
}

[data-theme="light"] .time-display {
  color: rgba(0, 0, 0, 0.6);
}

[data-theme="light"] .progress-track {
  background: rgba(0, 0, 0, 0.08);
}

[data-theme="light"] .track-btn {
  background: rgba(0, 0, 0, 0.03);
  border-color: rgba(0, 0, 0, 0.1);
  color: rgba(0, 0, 0, 0.6);
}

[data-theme="light"] .track-btn:hover {
  color: rgba(0, 0, 0, 0.9);
}

[data-theme="light"] .track-btn.active {
  background: linear-gradient(135deg, #1DB954 0%, #1ed760 100%);
  color: white;
  border-color: transparent;
}

[data-theme="light"] .volume-slider {
  background: rgba(0, 0, 0, 0.08);
}

[data-theme="light"] .volume-label {
  color: rgba(0, 0, 0, 0.6);
}

[data-theme="light"] .volume-section {
  border-top-color: rgba(0, 0, 0, 0.08);
}
</style>
