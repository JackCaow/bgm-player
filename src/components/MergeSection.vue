<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { useI18n } from "vue-i18n";

const { t } = useI18n();

defineProps<{
  bgmVolume: number;
  vocalsVolume: number;
  isMerging: boolean;
  mergedPath: string | null;
}>();

const emit = defineEmits<{
  "update:bgmVolume": [value: number];
  "update:vocalsVolume": [value: number];
  merge: [];
  reset: [];
}>();
</script>

<template>
  <div class="merge-section">
    <div class="merge-header">
      <Icon icon="solar:layers-bold-duotone" width="20" />
      <span>{{ t("merge.title") }}</span>
      <button class="reset-btn" @click="emit('reset')">
        <Icon icon="solar:restart-bold" width="16" />
        {{ t("merge.reset") }}
      </button>
    </div>
    <div class="volume-controls">
      <div class="volume-control">
        <div class="volume-label">
          <Icon icon="solar:music-note-2-bold-duotone" width="16" class="icon-bgm" />
          <span>{{ t("result.bgm") }}</span>
          <span class="volume-value">{{ Math.round(bgmVolume * 100) }}%</span>
        </div>
        <input
          type="range"
          :value="bgmVolume"
          @input="emit('update:bgmVolume', Number(($event.target as HTMLInputElement).value))"
          min="0"
          max="2"
          step="0.05"
          class="volume-slider"
        />
      </div>
      <div class="volume-control">
        <div class="volume-label">
          <Icon icon="solar:microphone-3-bold-duotone" width="16" class="icon-vocals" />
          <span>{{ t("result.vocals") }}</span>
          <span class="volume-value">{{ Math.round(vocalsVolume * 100) }}%</span>
        </div>
        <input
          type="range"
          :value="vocalsVolume"
          @input="emit('update:vocalsVolume', Number(($event.target as HTMLInputElement).value))"
          min="0"
          max="2"
          step="0.05"
          class="volume-slider"
        />
      </div>
    </div>
    <button class="merge-btn" :disabled="isMerging" @click="emit('merge')">
      <template v-if="!isMerging">
        <Icon icon="solar:layers-bold" width="18" />
        {{ t("merge.mergeButton") }}
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
