import { ref, computed } from "vue";
import type { HistoryItem, FileItem } from "@/types";

const STORAGE_KEY = "bgm-history";
const MAX_HISTORY_ITEMS = 100;

export function useHistory() {
  const history = ref<HistoryItem[]>([]);
  const selectedHistoryId = ref<string | null>(null);

  const historyCount = computed(() => history.value.length);

  const selectedHistory = computed(() => {
    if (!selectedHistoryId.value) return null;
    return history.value.find((h) => h.id === selectedHistoryId.value) || null;
  });

  function loadHistory() {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved) {
        const parsed = JSON.parse(saved);
        // Migrate old format to new format
        history.value = parsed.map(migrateHistoryItem);
      }
    } catch (e) {
      console.error("Failed to load history:", e);
    }
  }

  // Migrate old history items to new multi-track format
  function migrateHistoryItem(item: any): HistoryItem {
    // If already in new format with track_type, return as is
    if (item.tracks && item.mode && item.tracks[0]?.track_type) {
      return item;
    }

    // If has tracks but with old 'type' field, convert to track_type
    if (item.tracks && item.mode) {
      return {
        ...item,
        tracks: item.tracks.map((t: any) => ({
          track_type: t.track_type || t.type,
          path: t.path,
          name: t.name,
        })),
      };
    }

    // If in old format (only bgmPath and vocalsPath), convert to new format
    if (item.bgmPath && item.vocalsPath) {
      return {
        ...item,
        mode: "2-track",
        tracks: [
          {
            track_type: "vocals",
            path: item.vocalsPath,
            name: "人声",
          },
          {
            track_type: "no_vocals",
            path: item.bgmPath,
            name: "BGM",
          },
        ],
      };
    }

    // Fallback: return item as is
    return item;
  }

  function saveHistory() {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(history.value));
    } catch (e) {
      console.error("Failed to save history:", e);
    }
  }

  function addToHistory(file: FileItem, model: string) {
    if (!file.result) return;

    const processingTime = file.startTime
      ? Math.round((Date.now() - file.startTime) / 1000)
      : undefined;

    const item: HistoryItem = {
      id: crypto.randomUUID(),
      name: file.name,
      sourcePath: file.path,
      mode: file.result.mode,
      tracks: file.result.tracks,
      model,
      processedAt: Date.now(),
      processingTime,
      durationSec: file.durationSec,
      // Backward compatibility
      bgmPath: file.result.bgmPath,
      vocalsPath: file.result.vocalsPath,
    };

    history.value.unshift(item);

    if (history.value.length > MAX_HISTORY_ITEMS) {
      history.value = history.value.slice(0, MAX_HISTORY_ITEMS);
    }

    saveHistory();
  }

  function deleteHistoryItem(id: string) {
    history.value = history.value.filter((h) => h.id !== id);
    if (selectedHistoryId.value === id) {
      selectedHistoryId.value = history.value[0]?.id || null;
    }
    saveHistory();
  }

  function clearHistory() {
    history.value = [];
    selectedHistoryId.value = null;
    saveHistory();
  }

  function selectHistoryItem(id: string) {
    selectedHistoryId.value = id;
  }

  return {
    history,
    historyCount,
    selectedHistoryId,
    selectedHistory,
    loadHistory,
    addToHistory,
    deleteHistoryItem,
    clearHistory,
    selectHistoryItem,
  };
}
