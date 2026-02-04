import { ref, computed, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type { FileItem, ProgressPayload, TabType, ExtractResult } from "@/types";
import { isAudioFile, getFileName, AUDIO_EXTENSIONS } from "@/utils/format";
import { useExportSettings } from "./useExportSettings";
import { useQueueManager } from "./useQueueManager";
import { useNotifications } from "./useNotifications";
import { useGPUSettings } from "./useGPUSettings";
import { useSeparationSettings } from "./useSeparationSettings";
import { useModelSettings } from "./useModelSettings";
import { parseError, formatErrorMessage } from "@/utils/errorHandler";

export function useFiles() {
  const files = ref<FileItem[]>([]);
  const outputDir = ref<string>("");
  const isProcessing = ref(false);
  const currentFileId = ref<string | null>(null);
  const activeTab = ref<TabType>("queue");
  const selectedFileId = ref<string | null>(null);

  const { exportSettings } = useExportSettings();
  const queueManager = useQueueManager();
  const { success, error: showError } = useNotifications();
  const { gpuSettings } = useGPUSettings();
  const { getEffectiveMode } = useSeparationSettings();
  const { model, updateModel } = useModelSettings();

  let unlistenProgress: UnlistenFn | null = null;
  let unlistenFileDrop: UnlistenFn | null = null;
  let processingLoop: ReturnType<typeof setTimeout> | null = null;

  const canProcess = computed(() => {
    return files.value.some((f) => f.status === "pending") && !isProcessing.value;
  });

  const pendingCount = computed(
    () => files.value.filter((f) => f.status === "pending").length
  );

  const doneCount = computed(
    () => files.value.filter((f) => f.status === "done").length
  );

  const queueStats = computed(() => queueManager.calculateStats(files.value));

  const selectedFile = computed(() => {
    if (!selectedFileId.value) return null;
    return files.value.find((f) => f.id === selectedFileId.value) || null;
  });

  function handleDroppedFiles(paths: string[]) {
    const newFiles = paths
      .filter((p) => isAudioFile(p))
      .filter((p) => !files.value.some((f) => f.path === p))
      .map((p) => ({
        id: crypto.randomUUID(),
        path: p,
        name: getFileName(p),
        status: "pending" as const,
        progress: 0,
      }));

    files.value.push(...newFiles);

    if (newFiles.length > 0 && !selectedFileId.value) {
      selectedFileId.value = newFiles[0].id;
    }
  }

  async function selectInputFiles() {
    try {
      const selected = await open({
        multiple: true,
        filters: [{ name: "Audio", extensions: AUDIO_EXTENSIONS }],
      });
      if (selected) {
        const paths = Array.isArray(selected) ? selected : [selected];
        handleDroppedFiles(paths);
      }
    } catch (e) {
      console.error("Failed to select files:", e);
    }
  }

  async function selectOutputDir() {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (selected) {
        outputDir.value = selected as string;
      }
    } catch (e) {
      console.error("Failed to select directory:", e);
    }
  }

  function removeFile(id: string) {
    files.value = files.value.filter((f) => f.id !== id);
    if (selectedFileId.value === id) {
      selectedFileId.value = files.value[0]?.id || null;
    }
  }

  function clearCompleted() {
    files.value = files.value.filter(
      (f) => f.status !== "done" && f.status !== "error"
    );
    if (selectedFileId.value && !files.value.find((f) => f.id === selectedFileId.value)) {
      selectedFileId.value = files.value[0]?.id || null;
    }
  }

  function selectFile(id: string) {
    selectedFileId.value = id;
    const file = files.value.find((f) => f.id === id);
    if (file?.status === "done") {
      activeTab.value = "result";
    } else {
      activeTab.value = "queue";
    }
  }

  // Process a single file
  async function processSingleFile(file: FileItem) {
    queueManager.markAsProcessing(file);
    currentFileId.value = file.id;
    selectedFileId.value = file.id;

    try {
      const effectiveMode = getEffectiveMode(model.value);
      const response = await invoke<ExtractResult>(
        "extract_bgm",
        {
          input: file.path,
          output: outputDir.value || null,
          model: model.value,
          format: exportSettings.value.format,
          quality: exportSettings.value.quality,
          bitrate: exportSettings.value.bitrate,
          sampleRate: exportSettings.value.sampleRate,
          useGpu: gpuSettings.value.enabled,
          separationMode: effectiveMode,
        }
      );

      file.result = response;
      queueManager.markAsCompleted(file);

      // Show success notification
      success(
        "处理完成",
        `${file.name} 已成功提取背景音乐`,
        3000
      );
    } catch (e) {
      const errorDetails = parseError(e);
      const errorMessage = formatErrorMessage(errorDetails);

      queueManager.markAsFailed(file, errorMessage);

      // Show error notification
      showError(
        "处理失败",
        errorMessage,
        errorDetails.canRetry ? 8000 : 10000
      );
    }
  }

  // Main processing loop with concurrent support
  async function startBatchExtraction(onFileProcessed?: (file: FileItem) => void) {
    if (!canProcess.value || isProcessing.value) return;

    isProcessing.value = true;

    // Start processing loop
    processingLoop = setInterval(async () => {
      // Check if paused
      if (queueManager.isPaused.value) {
        return;
      }

      // Get next files to process
      const nextFiles = queueManager.getNextFilesToProcess(files.value);

      // Process each file concurrently
      for (const file of nextFiles) {
        processSingleFile(file).then(() => {
          onFileProcessed?.(file);

          // Check if all done
          const hasMorePending = files.value.some(
            (f) => f.status === "pending" || f.status === "processing"
          );

          if (!hasMorePending) {
            stopProcessing();

            // Select last completed file
            const lastDone = files.value.filter((f) => f.status === "done").pop();
            if (lastDone) {
              selectedFileId.value = lastDone.id;
              activeTab.value = "result";
            }
          }
        });
      }
    }, 1000); // Check every second
  }

  // Stop processing loop
  function stopProcessing() {
    if (processingLoop) {
      clearInterval(processingLoop);
      processingLoop = null;
    }
    isProcessing.value = false;
    currentFileId.value = null;
  }

  // Pause queue
  function pauseQueue() {
    queueManager.pauseQueue(files.value);
  }

  // Resume queue
  function resumeQueue() {
    queueManager.resumeQueue(files.value);
    if (!isProcessing.value && canProcess.value) {
      startBatchExtraction();
    }
  }

  // Cancel specific file
  function cancelFile(fileId: string) {
    const file = files.value.find((f) => f.id === fileId);
    if (file) {
      queueManager.cancelFile(file);
    }
  }

  // Cancel all files
  function cancelAll() {
    queueManager.cancelAll(files.value);
    stopProcessing();
  }

  // Retry failed file
  function retryFile(fileId: string) {
    const file = files.value.find((f) => f.id === fileId);
    if (file) {
      queueManager.retryFile(file);
      if (!isProcessing.value) {
        startBatchExtraction();
      }
    }
  }

  // Retry all failed files
  function retryAllFailed() {
    queueManager.retryAllFailed(files.value);
    if (!isProcessing.value && canProcess.value) {
      startBatchExtraction();
    }
  }

  // Move file up in priority
  function moveFileUp(fileId: string) {
    queueManager.moveFileUp(files.value, fileId);
  }

  // Move file down in priority
  function moveFileDown(fileId: string) {
    queueManager.moveFileDown(files.value, fileId);
  }

  async function setupListeners() {
    unlistenProgress = await listen<ProgressPayload>(
      "extraction-progress",
      (event) => {
        if (currentFileId.value) {
          const file = files.value.find((f) => f.id === currentFileId.value);
          if (file) {
            file.progress = event.payload.progress;
          }
        }
      }
    );

    unlistenFileDrop = await listen<{ paths: string[] }>(
      "tauri://drag-drop",
      (event) => {
        handleDroppedFiles(event.payload.paths);
      }
    );
  }

  function cleanupListeners() {
    unlistenProgress?.();
    unlistenFileDrop?.();
    stopProcessing();
  }

  onMounted(() => {
    setupListeners();
  });

  onUnmounted(() => {
    cleanupListeners();
  });

  return {
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
    queueStats,
    queueSettings: queueManager.queueSettings,
    isPaused: queueManager.isPaused,
    handleDroppedFiles,
    selectInputFiles,
    selectOutputDir,
    removeFile,
    clearCompleted,
    selectFile,
    startBatchExtraction,
    pauseQueue,
    resumeQueue,
    cancelFile,
    cancelAll,
    retryFile,
    retryAllFailed,
    moveFileUp,
    moveFileDown,
    updateQueueSettings: queueManager.updateSettings,
  };
}
