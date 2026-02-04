<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { useI18n } from "vue-i18n";
import type { FileItem } from "@/types";
import { formatPath } from "@/utils/format";

const { t } = useI18n();

defineProps<{
  selectedFile: FileItem | null;
  pendingCount: number;
}>();

const emit = defineEmits<{
  addFiles: [];
}>();
</script>

<template>
  <div class="content-panel">
    <div class="panel-header">
      <h2>{{ t("queue.title") }}</h2>
      <span class="panel-subtitle">{{ t("queue.pending", { count: pendingCount }) }}</span>
    </div>

    <div v-if="!selectedFile" class="empty-content">
      <div class="drop-zone" @click="emit('addFiles')">
        <Icon icon="solar:cloud-upload-bold-duotone" width="56" />
        <span class="drop-title">{{ t("queue.dropTitle") }}</span>
        <span class="drop-subtitle">{{ t("queue.dropSubtitle") }}</span>
        <span class="drop-formats">{{ t("queue.dropFormats") }}</span>
      </div>
    </div>

    <div v-else class="file-detail">
      <div class="detail-card">
        <div class="detail-header">
          <div class="detail-icon" :class="selectedFile.status">
            <Icon
              v-if="selectedFile.status === 'pending'"
              icon="solar:clock-circle-bold-duotone"
              width="28"
            />
            <div v-else-if="selectedFile.status === 'processing'" class="spinner"></div>
            <Icon
              v-else-if="selectedFile.status === 'error'"
              icon="solar:close-circle-bold-duotone"
              width="28"
            />
          </div>
          <div class="detail-info">
            <h3>{{ selectedFile.name }}</h3>
            <span class="detail-path">{{ formatPath(selectedFile.path) }}</span>
          </div>
        </div>

        <div v-if="selectedFile.status === 'processing'" class="progress-section">
          <div class="progress-bar">
            <div class="progress-fill" :style="{ width: selectedFile.progress + '%' }"></div>
          </div>
          <span class="progress-text">{{ Math.round(selectedFile.progress) }}%</span>
        </div>

        <div v-if="selectedFile.status === 'error'" class="error-section">
          <span>{{ selectedFile.error }}</span>
        </div>

        <div v-if="selectedFile.status === 'pending'" class="status-section">
          <span class="status-badge pending">{{ t("queue.waitingProcess") }}</span>
        </div>
      </div>
    </div>
  </div>
</template>
