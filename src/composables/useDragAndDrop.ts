import { ref, onMounted, onUnmounted } from "vue";

export function useDragAndDrop(onFilesDropped: (files: string[]) => void) {
  const isDragging = ref(false);
  const dragCounter = ref(0);

  function handleDragEnter(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();

    dragCounter.value++;

    if (e.dataTransfer?.types.includes("Files")) {
      isDragging.value = true;
    }
  }

  function handleDragLeave(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();

    dragCounter.value--;

    if (dragCounter.value === 0) {
      isDragging.value = false;
    }
  }

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();

    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = "copy";
    }
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();

    isDragging.value = false;
    dragCounter.value = 0;

    const files = e.dataTransfer?.files;
    if (files && files.length > 0) {
      const filePaths = Array.from(files).map(file => (file as any).path || file.name);
      onFilesDropped(filePaths);
    }
  }

  function setupDragListeners() {
    document.addEventListener("dragenter", handleDragEnter);
    document.addEventListener("dragleave", handleDragLeave);
    document.addEventListener("dragover", handleDragOver);
    document.addEventListener("drop", handleDrop);
  }

  function cleanupDragListeners() {
    document.removeEventListener("dragenter", handleDragEnter);
    document.removeEventListener("dragleave", handleDragLeave);
    document.removeEventListener("dragover", handleDragOver);
    document.removeEventListener("drop", handleDrop);
  }

  onMounted(() => {
    setupDragListeners();
  });

  onUnmounted(() => {
    cleanupDragListeners();
  });

  return {
    isDragging,
  };
}
