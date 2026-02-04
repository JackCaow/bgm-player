<script setup lang="ts">
import { ref, computed } from "vue";
import { Icon } from "@iconify/vue";
import { useI18n } from "vue-i18n";
import { useProjects } from "@/composables/useProjects";
import { useConfirm } from "@/composables/useConfirm";
import { useNotifications } from "@/composables/useNotifications";
import type { Project } from "@/types";

const { t } = useI18n();
const {
  projects,
  currentProject,
  createProject,
  loadProject,
  deleteProject,
  renameProject,
  exportProject,
  importProject,
} = useProjects();
const { confirm } = useConfirm();
const { success, error: showError } = useNotifications();

const emit = defineEmits<{
  close: [];
  projectLoaded: [project: Project];
}>();

const showNewDialog = ref(false);
const showRenameDialog = ref(false);
const showExportDialog = ref(false);
const showImportDialog = ref(false);
const newProjectName = ref("");
const renameProjectName = ref("");
const selectedProjectForRename = ref<Project | null>(null);
const selectedProjectForExport = ref<Project | null>(null);
const exportedProjectJson = ref("");
const importProjectJson = ref("");

const sortedProjects = computed(() => {
  return [...projects.value].sort((a, b) => b.lastModified - a.lastModified);
});

function handleNewProject() {
  showNewDialog.value = true;
}

function confirmNewProject() {
  if (!newProjectName.value.trim()) return;

  try {
    // Get current config from props or use defaults
    const config = {
      model: "htdemucs",
      outputDir: "",
      exportSettings: {
        format: "mp3" as const,
        quality: "high" as const,
        bitrate: 192,
      },
      separationSettings: {
        mode: "2-track" as const,
        autoSelectMode: true,
      },
      gpuSettings: {
        enabled: false,
        device: "cpu" as const,
        autoDetect: true,
      },
      queueSettings: {
        maxConcurrent: 2,
        autoRetry: true,
        maxRetries: 3,
        pauseOnError: false,
      },
    };

    const project = createProject(newProjectName.value.trim(), config);
    success(t("project.title"), `${project.name} 已创建`);
    showNewDialog.value = false;
    newProjectName.value = "";
  } catch (e) {
    showError(t("project.title"), "创建失败");
  }
}

async function handleLoadProject(project: Project) {
  if (currentProject.value?.id === project.id) {
    emit("close");
    return;
  }

  const confirmed = await confirm({
    title: t("project.switchConfirm.title"),
    message: t("project.switchConfirm.message")
  });

  if (confirmed) {
    try {
      loadProject(project.id);
      emit("projectLoaded", project);
      success(t("project.title"), `已切换到 ${project.name}`);
      emit("close");
    } catch (e) {
      showError(t("project.title"), "加载失败");
    }
  }
}

function handleRenameProject(project: Project) {
  selectedProjectForRename.value = project;
  renameProjectName.value = project.name;
  showRenameDialog.value = true;
}

function confirmRenameProject() {
  if (!renameProjectName.value.trim() || !selectedProjectForRename.value) return;

  try {
    renameProject(selectedProjectForRename.value.id, renameProjectName.value.trim());
    success(t("project.title"), "重命名成功");
    showRenameDialog.value = false;
    renameProjectName.value = "";
    selectedProjectForRename.value = null;
  } catch (e) {
    showError(t("project.title"), "重命名失败");
  }
}

async function handleDeleteProject(project: Project) {
  const confirmed = await confirm({
    title: t("project.deleteConfirm.title"),
    message: t("project.deleteConfirm.message")
  });

  if (confirmed) {
    try {
      deleteProject(project.id);
      success(t("project.title"), `${project.name} 已删除`);
    } catch (e) {
      showError(t("project.title"), "删除失败");
    }
  }
}

function handleExportProject(project: Project) {
  try {
    selectedProjectForExport.value = project;
    exportedProjectJson.value = exportProject(project.id);
    showExportDialog.value = true;
  } catch (e) {
    showError(t("project.title"), "导出失败");
  }
}

function copyToClipboard() {
  navigator.clipboard.writeText(exportedProjectJson.value);
  success(t("project.title"), "已复制到剪贴板");
}

function downloadProject() {
  const blob = new Blob([exportedProjectJson.value], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `${selectedProjectForExport.value?.name || "project"}.bgm-project.json`;
  a.click();
  URL.revokeObjectURL(url);
  success(t("project.title"), "项目已下载");
}

function handleImportProject() {
  try {
    const project = importProject(importProjectJson.value);
    success(t("project.title"), `${project.name} 已导入`);
    showImportDialog.value = false;
    importProjectJson.value = "";
  } catch (e) {
    showError(t("project.title"), "导入失败：格式无效");
  }
}

function formatDate(timestamp: number): string {
  return new Date(timestamp).toLocaleString();
}

function formatDuration(ms: number): string {
  const seconds = Math.floor(ms / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);

  if (hours > 0) {
    return `${hours}h ${minutes % 60}m`;
  } else if (minutes > 0) {
    return `${minutes}m ${seconds % 60}s`;
  } else {
    return `${seconds}s`;
  }
}
</script>

<template>
  <div class="project-manager">
    <div class="manager-header">
      <h2>{{ t("project.title") }}</h2>
      <button class="close-btn" @click="emit('close')">
        <Icon icon="solar:close-circle-linear" width="24" />
      </button>
    </div>

    <div class="manager-toolbar">
      <button class="toolbar-btn primary" @click="handleNewProject">
        <Icon icon="solar:add-circle-linear" width="20" />
        {{ t("project.new") }}
      </button>
      <button class="toolbar-btn" @click="showImportDialog = true">
        <Icon icon="solar:import-linear" width="20" />
        {{ t("project.import") }}
      </button>
    </div>

    <div class="manager-content">
      <div v-if="projects.length === 0" class="empty-state">
        <Icon icon="solar:folder-open-linear" width="64" />
        <p>暂无项目</p>
        <p class="hint">创建新项目开始使用</p>
      </div>

      <div v-else class="project-list">
        <div
          v-for="project in sortedProjects"
          :key="project.id"
          class="project-item"
          :class="{ active: currentProject?.id === project.id }"
        >
          <div class="project-main" @click="handleLoadProject(project)">
            <div class="project-icon">
              <Icon icon="solar:folder-bold-duotone" width="32" />
            </div>
            <div class="project-info">
              <div class="project-name">
                {{ project.name }}
                <span v-if="currentProject?.id === project.id" class="current-badge">
                  {{ t("project.current") }}
                </span>
              </div>
              <div class="project-meta">
                <span>
                  <Icon icon="solar:file-text-linear" width="14" />
                  {{ project.files.length }} {{ t("project.files") }}
                </span>
                <span>
                  <Icon icon="solar:clock-circle-linear" width="14" />
                  {{ formatDate(project.lastModified) }}
                </span>
              </div>
              <div class="project-stats">
                <span class="stat-item success">
                  <Icon icon="solar:check-circle-linear" width="14" />
                  {{ project.stats.totalProcessed }}
                </span>
                <span class="stat-item error">
                  <Icon icon="solar:close-circle-linear" width="14" />
                  {{ project.stats.totalFailed }}
                </span>
                <span v-if="project.stats.totalProcessingTime > 0" class="stat-item">
                  <Icon icon="solar:hourglass-linear" width="14" />
                  {{ formatDuration(project.stats.totalProcessingTime) }}
                </span>
              </div>
            </div>
          </div>
          <div class="project-actions">
            <button
              class="action-btn"
              @click.stop="handleRenameProject(project)"
              :title="t('project.rename')"
            >
              <Icon icon="solar:pen-linear" width="18" />
            </button>
            <button
              class="action-btn"
              @click.stop="handleExportProject(project)"
              :title="t('project.export')"
            >
              <Icon icon="solar:export-linear" width="18" />
            </button>
            <button
              class="action-btn danger"
              @click.stop="handleDeleteProject(project)"
              :title="t('project.delete')"
            >
              <Icon icon="solar:trash-bin-trash-linear" width="18" />
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- New Project Dialog -->
    <div v-if="showNewDialog" class="dialog-overlay" @click="showNewDialog = false">
      <div class="dialog-content" @click.stop>
        <div class="dialog-header">
          <h3>{{ t("project.newDialog.title") }}</h3>
          <button class="dialog-close" @click="showNewDialog = false">
            <Icon icon="solar:close-circle-linear" width="24" />
          </button>
        </div>
        <div class="dialog-body">
          <div class="form-group">
            <label>{{ t("project.name") }}</label>
            <input
              v-model="newProjectName"
              type="text"
              :placeholder="t('project.newDialog.namePlaceholder')"
              class="form-input"
              @keyup.enter="confirmNewProject"
            />
          </div>
        </div>
        <div class="dialog-footer">
          <button class="btn-secondary" @click="showNewDialog = false">
            {{ t("confirm.cancel") }}
          </button>
          <button
            class="btn-primary"
            @click="confirmNewProject"
            :disabled="!newProjectName.trim()"
          >
            {{ t("confirm.confirm") }}
          </button>
        </div>
      </div>
    </div>

    <!-- Rename Project Dialog -->
    <div v-if="showRenameDialog" class="dialog-overlay" @click="showRenameDialog = false">
      <div class="dialog-content" @click.stop>
        <div class="dialog-header">
          <h3>{{ t("project.renameDialog.title") }}</h3>
          <button class="dialog-close" @click="showRenameDialog = false">
            <Icon icon="solar:close-circle-linear" width="24" />
          </button>
        </div>
        <div class="dialog-body">
          <div class="form-group">
            <label>{{ t("project.name") }}</label>
            <input
              v-model="renameProjectName"
              type="text"
              :placeholder="t('project.renameDialog.namePlaceholder')"
              class="form-input"
              @keyup.enter="confirmRenameProject"
            />
          </div>
        </div>
        <div class="dialog-footer">
          <button class="btn-secondary" @click="showRenameDialog = false">
            {{ t("confirm.cancel") }}
          </button>
          <button
            class="btn-primary"
            @click="confirmRenameProject"
            :disabled="!renameProjectName.trim()"
          >
            {{ t("confirm.confirm") }}
          </button>
        </div>
      </div>
    </div>

    <!-- Export Dialog -->
    <div v-if="showExportDialog" class="dialog-overlay" @click="showExportDialog = false">
      <div class="dialog-content" @click.stop>
        <div class="dialog-header">
          <h3>{{ t("project.export") }}</h3>
          <button class="dialog-close" @click="showExportDialog = false">
            <Icon icon="solar:close-circle-linear" width="24" />
          </button>
        </div>
        <div class="dialog-body">
          <p class="dialog-hint">复制以下内容或下载为文件</p>
          <textarea
            v-model="exportedProjectJson"
            readonly
            class="export-textarea"
            rows="12"
          ></textarea>
        </div>
        <div class="dialog-footer">
          <button class="btn-secondary" @click="copyToClipboard">
            <Icon icon="solar:copy-linear" width="18" />
            复制
          </button>
          <button class="btn-primary" @click="downloadProject">
            <Icon icon="solar:download-linear" width="18" />
            下载
          </button>
        </div>
      </div>
    </div>

    <!-- Import Dialog -->
    <div v-if="showImportDialog" class="dialog-overlay" @click="showImportDialog = false">
      <div class="dialog-content" @click.stop>
        <div class="dialog-header">
          <h3>{{ t("project.import") }}</h3>
          <button class="dialog-close" @click="showImportDialog = false">
            <Icon icon="solar:close-circle-linear" width="24" />
          </button>
        </div>
        <div class="dialog-body">
          <p class="dialog-hint">粘贴项目 JSON 内容</p>
          <textarea
            v-model="importProjectJson"
            placeholder='{"id": "...", "name": "...", ...}'
            class="import-textarea"
            rows="12"
          ></textarea>
        </div>
        <div class="dialog-footer">
          <button class="btn-secondary" @click="showImportDialog = false">
            {{ t("confirm.cancel") }}
          </button>
          <button
            class="btn-primary"
            @click="handleImportProject"
            :disabled="!importProjectJson.trim()"
          >
            {{ t("confirm.confirm") }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.project-manager {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: var(--bg);
  z-index: 1000;
  display: flex;
  flex-direction: column;
}

.manager-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 24px;
  border-bottom: 1px solid var(--border);
}

.manager-header h2 {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
  color: var(--text);
}

.close-btn {
  background: none;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  padding: 0;
  display: flex;
  align-items: center;
  transition: color 0.2s;
}

.close-btn:hover {
  color: var(--text);
}

.manager-toolbar {
  display: flex;
  gap: 12px;
  padding: 16px 24px;
  border-bottom: 1px solid var(--border);
}

.toolbar-btn {
  padding: 10px 20px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 8px;
  color: var(--text);
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
  display: flex;
  align-items: center;
  gap: 8px;
}

.toolbar-btn:hover {
  background: var(--bg-hover);
  border-color: var(--primary);
}

.toolbar-btn.primary {
  background: var(--primary);
  color: white;
  border-color: var(--primary);
}

.toolbar-btn.primary:hover {
  background: var(--primary-hover);
}

.manager-content {
  flex: 1;
  overflow-y: auto;
  padding: 24px;
}

.empty-state {
  text-align: center;
  padding: 64px 24px;
  color: var(--text-secondary);
}

.empty-state p {
  margin: 12px 0;
  font-size: 16px;
}

.empty-state .hint {
  font-size: 14px;
  opacity: 0.7;
}

.project-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.project-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: var(--bg-card);
  border: 2px solid var(--border);
  border-radius: 12px;
  transition: all 0.2s;
  overflow: hidden;
}

.project-item:hover {
  border-color: var(--primary);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.project-item.active {
  border-color: var(--primary);
  background: rgba(29, 185, 84, 0.05);
}

.project-main {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 16px;
  cursor: pointer;
}

.project-icon {
  color: var(--primary);
  flex-shrink: 0;
}

.project-info {
  flex: 1;
  min-width: 0;
}

.project-name {
  font-size: 16px;
  font-weight: 600;
  color: var(--text);
  margin-bottom: 6px;
  display: flex;
  align-items: center;
  gap: 8px;
}

.current-badge {
  padding: 2px 8px;
  background: var(--primary);
  color: white;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 600;
}

.project-meta {
  display: flex;
  gap: 16px;
  font-size: 13px;
  color: var(--text-secondary);
  margin-bottom: 8px;
}

.project-meta span {
  display: flex;
  align-items: center;
  gap: 4px;
}

.project-stats {
  display: flex;
  gap: 12px;
}

.stat-item {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  color: var(--text-secondary);
}

.stat-item.success {
  color: var(--success);
}

.stat-item.error {
  color: var(--error);
}

.project-actions {
  display: flex;
  gap: 8px;
  padding: 16px;
  border-left: 1px solid var(--border);
}

.action-btn {
  padding: 8px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 8px;
  color: var(--text);
  cursor: pointer;
  transition: all 0.2s;
  display: flex;
  align-items: center;
  justify-content: center;
}

.action-btn:hover {
  background: var(--primary);
  color: white;
  border-color: var(--primary);
}

.action-btn.danger:hover {
  background: var(--error);
  border-color: var(--error);
}

/* Dialog Styles */
.dialog-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000;
  backdrop-filter: blur(4px);
}

.dialog-content {
  background: var(--bg);
  border-radius: 12px;
  width: 90%;
  max-width: 600px;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
}

.dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px;
  border-bottom: 1px solid var(--border);
}

.dialog-header h3 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  color: var(--text);
}

.dialog-close {
  background: none;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  padding: 0;
  display: flex;
  align-items: center;
  transition: color 0.2s;
}

.dialog-close:hover {
  color: var(--text);
}

.dialog-body {
  flex: 1;
  overflow-y: auto;
  padding: 20px;
}

.dialog-hint {
  margin: 0 0 12px 0;
  font-size: 13px;
  color: var(--text-secondary);
}

.form-group {
  margin-bottom: 16px;
}

.form-group:last-child {
  margin-bottom: 0;
}

.form-group label {
  display: block;
  margin-bottom: 8px;
  font-size: 13px;
  font-weight: 500;
  color: var(--text);
}

.form-input {
  width: 100%;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-card);
  color: var(--text);
  font-size: 13px;
  font-family: inherit;
  transition: all 0.2s;
}

.form-input:focus {
  outline: none;
  border-color: var(--primary);
  box-shadow: 0 0 0 3px rgba(29, 185, 84, 0.1);
}

.export-textarea,
.import-textarea {
  width: 100%;
  padding: 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-card);
  color: var(--text);
  font-size: 12px;
  font-family: "Monaco", "Menlo", "Courier New", monospace;
  resize: vertical;
  min-height: 200px;
}

.export-textarea:focus,
.import-textarea:focus {
  outline: none;
  border-color: var(--primary);
  box-shadow: 0 0 0 3px rgba(29, 185, 84, 0.1);
}

.dialog-footer {
  display: flex;
  gap: 12px;
  justify-content: flex-end;
  padding: 20px;
  border-top: 1px solid var(--border);
}

.btn-secondary,
.btn-primary {
  padding: 10px 20px;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
  border: none;
  display: flex;
  align-items: center;
  gap: 6px;
}

.btn-secondary {
  background: var(--bg-card);
  color: var(--text);
  border: 1px solid var(--border);
}

.btn-secondary:hover {
  background: var(--bg-hover);
}

.btn-primary {
  background: var(--primary);
  color: white;
}

.btn-primary:hover:not(:disabled) {
  background: var(--primary-hover);
}

.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
