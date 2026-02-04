<script setup lang="ts">
import { onMounted } from "vue";
import { Icon } from "@iconify/vue";
import { useI18n } from "vue-i18n";
import { useTheme } from "@/composables/useTheme";
import { useHistory } from "@/composables/useHistory";
import { useFiles } from "@/composables/useFiles";
import { useMerge } from "@/composables/useMerge";
import { useDragAndDrop } from "@/composables/useDragAndDrop";
import { useConfirm } from "@/composables/useConfirm";
import {
  AppSidebar,
  QueueView,
  ResultView,
  HistoryView,
  MergeView,
  SettingsView,
} from "@/components";
import NotificationContainer from "@/components/NotificationContainer.vue";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import GlobalPlayer from "@/components/GlobalPlayer.vue";
import { useGlobalPlayer } from "@/composables/useGlobalPlayer";

const { t } = useI18n();
const { confirm } = useConfirm();
const { isVisible: showGlobalPlayer } = useGlobalPlayer();

function handleShowPlayer() {
  showGlobalPlayer.value = true;
}

// Composables
const { theme, setTheme } = useTheme();
const {
  history,
  historyCount,
  selectedHistoryId,
  selectedHistory,
  loadHistory,
  addToHistory,
  deleteHistoryItem,
  clearHistory,
  selectHistoryItem,
} = useHistory();
const {
  files,
  outputDir,
  model,
  updateModel,
  isProcessing,
  activeTab,
  selectedFileId,
  canProcess,
  pendingCount,
  doneCount,
  selectedFile,
  handleDroppedFiles,
  selectInputFiles,
  selectOutputDir,
  removeFile,
  clearCompleted,
  selectFile,
  startBatchExtraction,
} = useFiles();
const {
  // Multi-track
  tracks,
  canMerge: _canMerge,
  addTrack,
  removeTrack,
  updateTrackVolume,
  moveTrackUp,
  moveTrackDown,
  mergeAllTracks,
  mergeSelectedTracks,
  clearAllTracks,
  resetAllVolumes,
  // Legacy 2-track
  bgmVolume,
  vocalsVolume,
  mergeTracks,
  resetMergeVolumes,
  // Shared
  isMerging,
  mergedPath,
} = useMerge();

// Drag and drop
const { isDragging } = useDragAndDrop((filePaths) => {
  handleDroppedFiles(filePaths);
});

// Confirm dialog wrappers for dangerous operations
async function handleClearCompleted() {
  const confirmed = await confirm({
    title: t("confirm.clearCompleted.title"),
    message: t("confirm.clearCompleted.message"),
    confirmText: t("confirm.confirm"),
    cancelText: t("confirm.cancel"),
    type: "warning",
  });
  if (confirmed) {
    clearCompleted();
  }
}

async function handleClearHistory() {
  const confirmed = await confirm({
    title: t("confirm.clearHistory.title"),
    message: t("confirm.clearHistory.message"),
    confirmText: t("confirm.confirm"),
    cancelText: t("confirm.cancel"),
    type: "danger",
  });
  if (confirmed) {
    clearHistory();
  }
}

async function handleDeleteHistory(id: string) {
  const confirmed = await confirm({
    title: t("confirm.deleteHistory.title"),
    message: t("confirm.deleteHistory.message"),
    confirmText: t("confirm.confirm"),
    cancelText: t("confirm.cancel"),
    type: "warning",
  });
  if (confirmed) {
    deleteHistoryItem(id);
  }
}

async function handleRemoveFile(id: string) {
  const confirmed = await confirm({
    title: t("confirm.removeFile.title"),
    message: t("confirm.removeFile.message"),
    confirmText: t("confirm.confirm"),
    cancelText: t("confirm.cancel"),
    type: "warning",
  });
  if (confirmed) {
    removeFile(id);
  }
}

async function handleClearAllTracks() {
  const confirmed = await confirm({
    title: t("confirm.clearTracks.title"),
    message: t("confirm.clearTracks.message"),
    confirmText: t("confirm.confirm"),
    cancelText: t("confirm.cancel"),
    type: "warning",
  });
  if (confirmed) {
    clearAllTracks();
  }
}

// Initialize
onMounted(() => {
  loadHistory();
});

// Handle file processing with history
function handleStartProcess() {
  startBatchExtraction((file) => {
    addToHistory(file, model.value);
  });
}

// Merge handlers for Result/History views
function handleMerge() {
  if (selectedFile.value?.result?.bgmPath && selectedFile.value?.result?.vocalsPath) {
    mergeTracks(selectedFile.value.result.bgmPath, selectedFile.value.result.vocalsPath);
  }
}

function handleMergeMultiTrack(selectedTracks: { path: string; volume: number }[]) {
  mergeSelectedTracks(selectedTracks);
}

function handleHistoryMerge() {
  if (selectedHistory.value?.bgmPath && selectedHistory.value?.vocalsPath) {
    mergeTracks(selectedHistory.value.bgmPath, selectedHistory.value.vocalsPath);
  }
}
</script>

<template>
  <div class="app">
    <!-- macOS Title Bar Drag Region -->
    <div class="titlebar-drag-region"></div>

    <!-- Notifications -->
    <NotificationContainer />

    <!-- Confirm Dialog -->
    <ConfirmDialog />

    <!-- Global Audio Player -->
    <GlobalPlayer />

    <!-- Drag Overlay -->    <Transition name="drag-fade">
      <div v-if="isDragging" class="drag-overlay">
        <div class="drag-content">
          <Icon icon="solar:upload-bold-duotone" width="64" />
          <span>{{ $t("drag.release") }}</span>
        </div>
      </div>
    </Transition>

    <!-- Layout Container -->
    <div class="layout">
      <!-- Sidebar -->
      <AppSidebar
        v-model:active-tab="activeTab"
        :files="files"
        :selected-file-id="selectedFileId"
        :pending-count="pendingCount"
        :done-count="doneCount"
        :history-count="historyCount"
        :is-processing="isProcessing"
        :can-process="canProcess"
        @select-file="selectFile"
        @remove-file="handleRemoveFile"
        @add-files="selectInputFiles"
        @start-process="handleStartProcess"
        @show-player="handleShowPlayer"
      />

      <!-- Main Content -->
      <main class="main-content">
        <!-- Queue View -->
        <QueueView
          v-if="activeTab === 'queue'"
          :selected-file="selectedFile"
          :pending-count="pendingCount"
          @add-files="selectInputFiles"
        />

        <!-- Result View -->
        <ResultView
          v-if="activeTab === 'result'"
          :selected-file="selectedFile"
          :done-count="doneCount"
          :bgm-volume="bgmVolume"
          :vocals-volume="vocalsVolume"
          :is-merging="isMerging"
          :merged-path="mergedPath"
          @clear-completed="handleClearCompleted"
          @update:bgm-volume="bgmVolume = $event"
          @update:vocals-volume="vocalsVolume = $event"
          @merge="handleMerge"
          @merge-multi-track="handleMergeMultiTrack"
          @reset-merge="resetMergeVolumes"
        />

        <!-- History View -->
        <HistoryView
          v-if="activeTab === 'history'"
          :history="history"
          :history-count="historyCount"
          :selected-history-id="selectedHistoryId"
          :selected-history="selectedHistory"
          :bgm-volume="bgmVolume"
          :vocals-volume="vocalsVolume"
          :is-merging="isMerging"
          :merged-path="mergedPath"
          @clear-history="handleClearHistory"
          @select-history-item="selectHistoryItem"
          @delete-history-item="handleDeleteHistory"
          @update:bgm-volume="bgmVolume = $event"
          @update:vocals-volume="vocalsVolume = $event"
          @merge="handleHistoryMerge"
          @merge-multi-track="handleMergeMultiTrack"
          @reset-merge="resetMergeVolumes"
        />

        <!-- Merge View (Multi-track) -->
        <MergeView
          v-if="activeTab === 'merge'"
          :tracks="tracks"
          :is-merging="isMerging"
          :merged-path="mergedPath"
          @add-track="addTrack"
          @remove-track="removeTrack"
          @update-volume="updateTrackVolume"
          @move-up="moveTrackUp"
          @move-down="moveTrackDown"
          @merge="mergeAllTracks"
          @reset-volumes="resetAllVolumes"
          @clear="handleClearAllTracks"
        />

        <!-- Settings View -->
        <SettingsView
          v-if="activeTab === 'settings'"
          :theme="theme"
          :model="model"
          :output-dir="outputDir"
          @update:theme="setTheme"
          @update:model="updateModel"
          @select-output-dir="selectOutputDir"
        />
      </main>
    </div>
  </div>
</template>
