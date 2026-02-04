<script setup lang="ts">
import { computed } from "vue";
import { Icon } from "@iconify/vue";
import { useI18n } from "vue-i18n";
import { useGlobalPlayer } from "@/composables/useGlobalPlayer";
import type { TrackType } from "@/types";
import Slider from "./ui/Slider.vue";

const { t } = useI18n();

const {
  isPlaying,
  progress,
  formattedCurrentTime,
  formattedDuration,
  activeTracks,
  currentTracks,
  isVisible,
  togglePlayPause,
  seekToPercent,
  setTrackVolume,
  getTrackVolume,
  toggleTrack,
  hidePlayer,
} = useGlobalPlayer();

const effectiveTracks = computed(() => currentTracks.value);

function handleProgressChange(value: number) {
  seekToPercent(value);
}

function handleTrackVolumeChange(trackType: TrackType, value: number) {
  setTrackVolume(trackType, value / 100);
}

function getTrackVolumePercent(trackType: TrackType): number {
  return getTrackVolume(trackType) * 100;
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
  return activeTracks.value.has(trackType);
}
</script>

<template>
  <Transition name="player-slide">
    <div v-if="isVisible" class="global-player-overlay" @click.self="hidePlayer">
      <div class="global-player">
        <!-- Header -->
        <div class="player-header">
          <div class="player-title">
            <Icon icon="solar:music-note-slider-bold-duotone" width="20" />
            <span>{{ t('player.play') }}</span>
          </div>
          <button class="close-btn" @click="hidePlayer" :title="t('player.close')">
            <Icon icon="solar:close-circle-bold" width="20" />
          </button>
        </div>

        <!-- Play Button -->
        <div class="play-section">
          <button class="play-btn" @click="togglePlayPause">
            <Icon v-if="!isPlaying" icon="solar:play-bold" width="32" />
            <Icon v-else icon="solar:pause-bold" width="32" />
          </button>
        </div>

        <!-- Progress Bar -->
        <div class="progress-section">
          <Slider
            :model-value="progress"
            :min="0"
            :max="100"
            :step="0.1"
            @update:model-value="handleProgressChange"
          />
          <div class="time-row">
            <span class="time-display">{{ formattedCurrentTime }}</span>
            <span class="time-display">{{ formattedDuration }}</span>
          </div>
        </div>

        <!-- Track Selection -->
        <div class="tracks-section">
          <div class="section-title">Tracks</div>
          <div class="track-list">
            <button
              v-for="track in effectiveTracks"
              :key="track.track_type"
              class="track-item"
              :class="{ active: isTrackActive(track.track_type as TrackType) }"
              @click="toggleTrack(track.track_type as TrackType)"
            >
              <Icon :icon="getTrackIcon(track.track_type)" width="18" />
              <span class="track-name">{{ track.name }}</span>
              <Icon
                v-if="isTrackActive(track.track_type as TrackType)"
                icon="solar:check-circle-bold"
                width="16"
                class="check-icon"
              />
            </button>
          </div>
        </div>

        <!-- Volume Controls -->
        <div class="volume-section">
          <div class="section-title">Volume</div>
          <div class="volume-list">
            <div
              v-for="track in effectiveTracks"
              :key="track.track_type"
              class="volume-item"
            >
              <div class="volume-header">
                <Icon :icon="getTrackIcon(track.track_type)" width="14" />
                <span>{{ track.name }}</span>
                <span class="volume-value">{{ Math.round(getTrackVolumePercent(track.track_type as TrackType)) }}%</span>
              </div>
              <Slider
                :model-value="getTrackVolumePercent(track.track_type as TrackType)"
                :min="0"
                :max="200"
                :step="1"
                @update:model-value="(v) => handleTrackVolumeChange(track.track_type as TrackType, v)"
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.global-player-overlay {
  position: fixed;
  top: 38px;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 1000;
  background: rgba(0, 0, 0, 0.3);
  display: flex;
}

.global-player {
  width: 280px;
  height: 100%;
  background: var(--bg-card);
  border-right: 1px solid var(--border);
  box-shadow: 4px 0 24px rgba(0, 0, 0, 0.2);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.player-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px;
  border-bottom: 1px solid var(--border);
}

.player-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
  color: var(--text);
}

.close-btn {
  background: transparent;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  padding: 4px;
  border-radius: 50%;
  transition: all 0.2s;
  display: flex;
  align-items: center;
  justify-content: center;
}

.close-btn:hover {
  color: var(--text);
  background: var(--bg-hover);
}

.play-section {
  padding: 24px;
  display: flex;
  justify-content: center;
}

.play-btn {
  width: 72px;
  height: 72px;
  border: none;
  border-radius: 50%;
  background: linear-gradient(135deg, #1DB954 0%, #1ed760 100%);
  color: white;
  cursor: pointer;
  transition: all 0.2s;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 6px 20px rgba(29, 185, 84, 0.4);
}

.play-btn:hover {
  transform: scale(1.08);
  box-shadow: 0 8px 28px rgba(29, 185, 84, 0.5);
}

.progress-section {
  padding: 0 16px 16px;
}

.time-row {
  display: flex;
  justify-content: space-between;
  margin-top: 8px;
}

.time-display {
  font-size: 11px;
  color: var(--text-secondary);
  font-variant-numeric: tabular-nums;
}

.section-title {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-secondary);
  padding: 12px 16px 8px;
  border-top: 1px solid var(--border);
}

.tracks-section {
  flex-shrink: 0;
}

.track-list {
  padding: 0 8px 8px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.track-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.2s;
  text-align: left;
  width: 100%;
}

.track-item:hover {
  background: var(--bg-hover);
  color: var(--text);
}

.track-item.active {
  background: rgba(29, 185, 84, 0.15);
  color: var(--primary);
}

.track-name {
  flex: 1;
  font-size: 13px;
  font-weight: 500;
}

.check-icon {
  color: var(--primary);
}

.volume-section {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
}

.volume-list {
  padding: 0 16px 16px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.volume-item {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.volume-header {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text-secondary);
}

.volume-header span:first-of-type {
  flex: 1;
}

.volume-value {
  font-size: 11px;
  font-variant-numeric: tabular-nums;
  color: var(--text);
  font-weight: 600;
}

/* Transition */
.player-slide-enter-active,
.player-slide-leave-active {
  transition: opacity 0.2s ease;
}

.player-slide-enter-active .global-player,
.player-slide-leave-active .global-player {
  transition: transform 0.3s ease;
}

.player-slide-enter-from,
.player-slide-leave-to {
  opacity: 0;
}

.player-slide-enter-from .global-player,
.player-slide-leave-to .global-player {
  transform: translateX(-100%);
}

/* Light theme */
[data-theme="light"] .global-player-overlay {
  background: rgba(0, 0, 0, 0.2);
}

[data-theme="light"] .global-player {
  box-shadow: 4px 0 24px rgba(0, 0, 0, 0.1);
}
</style>
