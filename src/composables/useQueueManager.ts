import { ref } from "vue";
import type { FileItem, QueueSettings, QueueStats } from "@/types";

export function useQueueManager() {
  const queueSettings = ref<QueueSettings>({
    maxConcurrent: 2,
    autoRetry: true,
    maxRetries: 3,
    pauseOnError: false,
  });

  const isPaused = ref(false);
  const processingFiles = ref<Set<string>>(new Set());
  const completedTimes = ref<number[]>([]); // Store processing times for estimation

  // Calculate queue statistics
  function calculateStats(files: FileItem[]): QueueStats {
    const stats: QueueStats = {
      total: files.length,
      pending: 0,
      processing: 0,
      paused: 0,
      done: 0,
      error: 0,
      cancelled: 0,
      averageProcessingTime: 0,
      estimatedTotalTime: 0,
    };

    files.forEach((file) => {
      switch (file.status) {
        case "pending":
          stats.pending++;
          break;
        case "processing":
          stats.processing++;
          break;
        case "paused":
          stats.paused++;
          break;
        case "done":
          stats.done++;
          break;
        case "error":
          stats.error++;
          break;
        case "cancelled":
          stats.cancelled++;
          break;
      }
    });

    // Calculate average processing time from completed files
    if (completedTimes.value.length > 0) {
      const sum = completedTimes.value.reduce((a, b) => a + b, 0);
      stats.averageProcessingTime = sum / completedTimes.value.length / 1000; // Convert to seconds
    }

    // Estimate total time for remaining files
    const remainingFiles = stats.pending + stats.processing + stats.paused;
    if (stats.averageProcessingTime > 0 && remainingFiles > 0) {
      stats.estimatedTotalTime = stats.averageProcessingTime * remainingFiles;
    }

    return stats;
  }

  // Sort files by priority (higher priority first)
  function sortByPriority(files: FileItem[]): FileItem[] {
    return [...files].sort((a, b) => {
      const priorityA = a.priority ?? 0;
      const priorityB = b.priority ?? 0;
      return priorityB - priorityA;
    });
  }

  // Get next files to process based on concurrent limit
  function getNextFilesToProcess(files: FileItem[]): FileItem[] {
    const availableSlots = queueSettings.value.maxConcurrent - processingFiles.value.size;
    if (availableSlots <= 0 || isPaused.value) {
      return [];
    }

    const pendingFiles = files.filter((f) => f.status === "pending");
    const sortedFiles = sortByPriority(pendingFiles);
    return sortedFiles.slice(0, availableSlots);
  }

  // Pause the entire queue
  function pauseQueue(files: FileItem[]) {
    isPaused.value = true;
    // Mark currently processing files as paused
    files.forEach((file) => {
      if (file.status === "processing") {
        file.status = "paused";
      }
    });
    processingFiles.value.clear();
  }

  // Resume the queue
  function resumeQueue(files: FileItem[]) {
    isPaused.value = false;
    // Mark paused files back to pending
    files.forEach((file) => {
      if (file.status === "paused") {
        file.status = "pending";
        file.progress = 0;
      }
    });
  }

  // Cancel a specific file
  function cancelFile(file: FileItem) {
    if (file.status === "processing") {
      processingFiles.value.delete(file.id);
    }
    file.status = "cancelled";
    file.progress = 0;
  }

  // Cancel all pending and processing files
  function cancelAll(files: FileItem[]) {
    files.forEach((file) => {
      if (file.status === "pending" || file.status === "processing" || file.status === "paused") {
        cancelFile(file);
      }
    });
    processingFiles.value.clear();
    isPaused.value = false;
  }

  // Retry a failed file
  function retryFile(file: FileItem) {
    if (file.status === "error" || file.status === "cancelled") {
      file.status = "pending";
      file.progress = 0;
      file.error = undefined;
      file.retryCount = (file.retryCount ?? 0) + 1;
    }
  }

  // Retry all failed files
  function retryAllFailed(files: FileItem[]) {
    files.forEach((file) => {
      if (file.status === "error") {
        retryFile(file);
      }
    });
  }

  // Move file up in priority
  function moveFileUp(files: FileItem[], fileId: string) {
    const file = files.find((f) => f.id === fileId);
    if (file) {
      file.priority = (file.priority ?? 0) + 1;
    }
  }

  // Move file down in priority
  function moveFileDown(files: FileItem[], fileId: string) {
    const file = files.find((f) => f.id === fileId);
    if (file) {
      file.priority = (file.priority ?? 0) - 1;
    }
  }

  // Record completed file processing time
  function recordProcessingTime(file: FileItem) {
    if (file.startTime && file.endTime) {
      const processingTime = file.endTime - file.startTime;
      file.processingTime = processingTime;
      completedTimes.value.push(processingTime);

      // Keep only last 20 times for better estimation
      if (completedTimes.value.length > 20) {
        completedTimes.value.shift();
      }
    }
  }

  // Mark file as processing
  function markAsProcessing(file: FileItem) {
    file.status = "processing";
    file.startTime = Date.now();
    processingFiles.value.add(file.id);
  }

  // Mark file as completed
  function markAsCompleted(file: FileItem) {
    file.status = "done";
    file.endTime = Date.now();
    file.progress = 100;
    processingFiles.value.delete(file.id);
    recordProcessingTime(file);
  }

  // Mark file as failed
  function markAsFailed(file: FileItem, error: string) {
    file.status = "error";
    file.error = error;
    file.endTime = Date.now();
    processingFiles.value.delete(file.id);

    // Check if should pause on error
    if (queueSettings.value.pauseOnError) {
      isPaused.value = true;
    }

    // Check if should auto retry
    const retryCount = file.retryCount ?? 0;
    if (queueSettings.value.autoRetry && retryCount < queueSettings.value.maxRetries) {
      retryFile(file);
    }
  }

  // Update queue settings
  function updateSettings(newSettings: Partial<QueueSettings>) {
    queueSettings.value = { ...queueSettings.value, ...newSettings };
  }

  return {
    queueSettings,
    isPaused,
    processingFiles,
    calculateStats,
    sortByPriority,
    getNextFilesToProcess,
    pauseQueue,
    resumeQueue,
    cancelFile,
    cancelAll,
    retryFile,
    retryAllFailed,
    moveFileUp,
    moveFileDown,
    markAsProcessing,
    markAsCompleted,
    markAsFailed,
    updateSettings,
  };
}

