<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { useI18n } from "vue-i18n";
import type { FileItem, TabType } from "@/types";
import { useGlobalPlayer } from "@/composables/useGlobalPlayer";

const { t } = useI18n();
const { isPlaying, currentTracks } = useGlobalPlayer();

defineProps<{
  activeTab: TabType;
  files: FileItem[];
  selectedFileId: string | null;
  pendingCount: number;
  doneCount: number;
  historyCount: number;
  isProcessing: boolean;
  canProcess: boolean;
}>();

const emit = defineEmits<{
  "update:activeTab": [tab: TabType];
  selectFile: [id: string];
  removeFile: [id: string];
  addFiles: [];
  startProcess: [];
  showPlayer: [];
}>();
</script>

<template>
  <aside class="sidebar">
    <div class="sidebar-header">
      <div
        class="logo"
        :class="{ 'has-tracks': currentTracks.length > 0 }"
        @click="currentTracks.length > 0 ? emit('showPlayer') : null"
        :style="{ cursor: currentTracks.length > 0 ? 'pointer' : 'default' }"
      >
        <div class="vinyl-player">
          <div class="vinyl" :class="{ spinning: isPlaying }">
            <div class="vinyl-label">
              <div class="vinyl-hole"></div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <nav class="sidebar-nav">
      <button
        class="nav-item"
        :class="{ active: activeTab === 'queue' }"
        @click="emit('update:activeTab', 'queue')"
      >
        <Icon icon="solar:playlist-bold-duotone" width="20" />
        <span>{{ t("nav.queue") }}</span>
        <span v-if="pendingCount > 0" class="nav-badge">{{ pendingCount }}</span>
      </button>
      <button
        class="nav-item"
        :class="{ active: activeTab === 'result' }"
        @click="emit('update:activeTab', 'result')"
      >
        <Icon icon="solar:music-notes-bold-duotone" width="20" />
        <span>{{ t("nav.result") }}</span>
        <span v-if="doneCount > 0" class="nav-badge success">{{ doneCount }}</span>
      </button>
      <button
        class="nav-item"
        :class="{ active: activeTab === 'history' }"
        @click="emit('update:activeTab', 'history')"
      >
        <Icon icon="solar:history-bold-duotone" width="20" />
        <span>{{ t("nav.history") }}</span>
        <span v-if="historyCount > 0" class="nav-badge">{{ historyCount }}</span>
      </button>
      <button
        class="nav-item"
        :class="{ active: activeTab === 'merge' }"
        @click="emit('update:activeTab', 'merge')"
      >
        <Icon icon="solar:layers-bold-duotone" width="20" />
        <span>{{ t("nav.merge") }}</span>
      </button>
      <button
        class="nav-item"
        :class="{ active: activeTab === 'settings' }"
        @click="emit('update:activeTab', 'settings')"
      >
        <Icon icon="solar:settings-bold-duotone" width="20" />
        <span>{{ t("nav.settings") }}</span>
      </button>
    </nav>

    <div class="sidebar-files">
      <div class="sidebar-files-header">
        <span>{{ t("sidebar.fileList") }}</span>
        <button class="add-btn" @click="emit('addFiles')" :title="t('sidebar.fileList')">
          <Icon icon="solar:add-circle-bold" width="18" />
        </button>
      </div>
      <div class="file-list-compact">
        <div
          v-for="file in files"
          :key="file.id"
          class="file-item-compact"
          :class="{ active: selectedFileId === file.id, [file.status]: true }"
          @click="emit('selectFile', file.id)"
        >
          <div class="file-status-dot" :class="file.status"></div>
          <span class="file-name-compact">{{ file.name }}</span>
          <button
            v-if="file.status === 'pending'"
            class="file-remove-btn"
            @click.stop="emit('removeFile', file.id)"
          >
            <Icon icon="solar:close-circle-bold" width="16" />
          </button>
        </div>
        <div v-if="files.length === 0" class="empty-files">
          <span>{{ t("sidebar.emptyFiles") }}</span>
        </div>
      </div>
    </div>

    <div class="sidebar-footer">
      <button
        class="start-btn"
        :class="{ processing: isProcessing }"
        :disabled="!canProcess"
        @click="emit('startProcess')"
      >
        <template v-if="!isProcessing">
          <Icon icon="solar:play-bold" width="18" />
          {{ t("sidebar.startProcess") }}
        </template>
        <template v-else>
          <div class="spinner"></div>
          {{ t("sidebar.processing") }}
        </template>
      </button>
    </div>
  </aside>
</template>

<style scoped>
/* Vinyl Record Player Effect */
.vinyl-player {
  width: 40px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.vinyl {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  background:
    radial-gradient(circle at 30% 30%, rgba(255,255,255,0.1) 0%, transparent 50%),
    conic-gradient(
      from 0deg,
      #111 0deg,
      #222 30deg,
      #111 60deg,
      #1a1a1a 90deg,
      #222 120deg,
      #111 150deg,
      #1a1a1a 180deg,
      #222 210deg,
      #111 240deg,
      #1a1a1a 270deg,
      #222 300deg,
      #111 330deg,
      #1a1a1a 360deg
    );
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
  box-shadow:
    0 2px 10px rgba(0, 0, 0, 0.4),
    inset 0 0 0 1px rgba(255, 255, 255, 0.1);
}

.vinyl::before {
  content: '';
  position: absolute;
  width: 32px;
  height: 32px;
  border-radius: 50%;
  border: 1px solid rgba(40, 40, 40, 0.8);
  box-shadow: inset 0 0 2px rgba(0, 0, 0, 0.5);
}

.vinyl-label {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: linear-gradient(135deg, var(--primary) 0%, #15803d 100%);
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
  z-index: 1;
}

.vinyl-hole {
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background: #111;
  box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.8);
}

.vinyl.spinning {
  animation: vinylSpin 2s linear infinite;
}

@keyframes vinylSpin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

/* Light theme */
[data-theme="light"] .vinyl {
  background:
    radial-gradient(circle at 30% 30%, rgba(255,255,255,0.3) 0%, transparent 50%),
    conic-gradient(
      from 0deg,
      #333 0deg,
      #444 30deg,
      #333 60deg,
      #3a3a3a 90deg,
      #444 120deg,
      #333 150deg,
      #3a3a3a 180deg,
      #444 210deg,
      #333 240deg,
      #3a3a3a 270deg,
      #444 300deg,
      #333 330deg,
      #3a3a3a 360deg
    );
  box-shadow:
    0 2px 10px rgba(0, 0, 0, 0.2),
    inset 0 0 0 1px rgba(255, 255, 255, 0.2);
}
</style>
