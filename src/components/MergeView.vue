<script setup lang="ts">
import { computed } from "vue";
import { Icon } from "@iconify/vue";
import { useI18n } from "vue-i18n";
import type { MergeTrack } from "@/composables/useMerge";
import { formatPath } from "@/utils/format";

const { t } = useI18n();

const props = defineProps<{
  tracks: MergeTrack[];
  isMerging: boolean;
  mergedPath: string | null;
}>();

const emit = defineEmits<{
  addTrack: [];
  removeTrack: [id: string];
  updateVolume: [id: string, volume: number];
  moveUp: [id: string];
  moveDown: [id: string];
  merge: [];
  resetVolumes: [];
  clear: [];
}>();

const canMerge = computed(() => props.tracks.length >= 2 && !props.isMerging);
</script>

<template>
  <div class="content-panel">
    <div class="panel-header">
      <h2>{{ t("merge.title") }}</h2>
      <button v-if="tracks.length > 0" class="clear-btn" @click="emit('clear')">
        {{ t("merge.clearFiles") }}
      </button>
    </div>

    <div class="merge-standalone">
      <!-- Track List Section -->
      <div class="merge-tracks-section">
        <div class="section-header">
          <h3 class="section-title">
            <Icon icon="solar:playlist-bold-duotone" width="20" />
            {{ t("merge.trackList") }}
            <span class="track-count">{{ tracks.length }} {{ t("merge.tracksCount") }}</span>
          </h3>
          <button class="reset-btn" @click="emit('resetVolumes')" v-if="tracks.length > 0">
            <Icon icon="solar:restart-bold" width="16" />
            {{ t("merge.reset") }}
          </button>
        </div>

        <!-- Add Track Button -->
        <div class="add-track-area" @click="emit('addTrack')">
          <Icon icon="solar:add-circle-bold-duotone" width="32" />
          <span>{{ t("merge.addTrack") }}</span>
          <span class="add-track-hint">{{ t("merge.addTrackHint") }}</span>
        </div>

        <!-- Track List -->
        <div v-if="tracks.length > 0" class="track-list">
          <div
            v-for="(track, index) in tracks"
            :key="track.id"
            class="track-item"
          >
            <div class="track-order">
              <button
                class="order-btn"
                :disabled="index === 0"
                @click="emit('moveUp', track.id)"
              >
                <Icon icon="solar:alt-arrow-up-bold" width="14" />
              </button>
              <span class="order-number">{{ index + 1 }}</span>
              <button
                class="order-btn"
                :disabled="index === tracks.length - 1"
                @click="emit('moveDown', track.id)"
              >
                <Icon icon="solar:alt-arrow-down-bold" width="14" />
              </button>
            </div>

            <div class="track-info">
              <span class="track-name">{{ track.name }}</span>
              <span class="track-path">{{ formatPath(track.path) }}</span>
            </div>

            <div class="track-volume">
              <input
                type="range"
                :value="track.volume"
                @input="emit('updateVolume', track.id, Number(($event.target as HTMLInputElement).value))"
                min="0"
                max="2"
                step="0.05"
                class="volume-slider-mini"
              />
              <span class="volume-value-mini">{{ Math.round(track.volume * 100) }}%</span>
            </div>

            <button class="track-remove" @click="emit('removeTrack', track.id)">
              <Icon icon="solar:trash-bin-minimalistic-bold" width="16" />
            </button>
          </div>
        </div>

        <!-- Empty State -->
        <div v-if="tracks.length === 0" class="tracks-empty">
          <Icon icon="solar:music-notes-bold-duotone" width="48" />
          <span>{{ t("merge.emptyTracks") }}</span>
        </div>
      </div>

      <!-- Merge Button -->
      <button class="merge-btn-large" :disabled="!canMerge" @click="emit('merge')">
        <template v-if="!isMerging">
          <Icon icon="solar:layers-bold" width="22" />
          {{ t("merge.mergeButton") }}
          <span v-if="tracks.length >= 2" class="merge-count">({{ tracks.length }} {{ t("merge.tracksCount") }})</span>
        </template>
        <template v-else>
          <div class="spinner"></div>
          {{ t("merge.merging") }}
        </template>
      </button>

      <!-- Result -->
      <div v-if="mergedPath" class="merge-result-standalone">
        <div class="merge-result-header">
          <Icon icon="solar:check-circle-bold" width="24" class="icon-success" />
          <div class="merge-result-info">
            <span class="merge-result-title">{{ t("merge.success") }}</span>
            <span class="merge-result-path">{{ mergedPath }}</span>
          </div>
        </div>
        <audio controls :src="'asset://localhost/' + mergedPath" class="merged-audio-large"></audio>
      </div>
    </div>
  </div>
</template>
