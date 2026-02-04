import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { AUDIO_EXTENSIONS, getFileName } from "@/utils/format";
import type { TrackInfo } from "@/types";

export interface MergeTrack {
  id: string;
  path: string;
  name: string;
  volume: number;
  enabled: boolean;
}

export function useMerge() {
  const tracks = ref<MergeTrack[]>([]);
  const isMerging = ref(false);
  const mergedPath = ref<string | null>(null);

  // Legacy support for 2-track merge in Result/History views
  const bgmVolume = ref(1.0);
  const vocalsVolume = ref(1.0);

  const canMerge = computed(() => {
    const enabledTracks = tracks.value.filter(t => t.enabled);
    return enabledTracks.length >= 2 && !isMerging.value;
  });

  // Load tracks from extraction result
  function loadTracksFromResult(trackInfos: TrackInfo[]) {
    tracks.value = trackInfos.map((info) => ({
      id: crypto.randomUUID(),
      path: info.path,
      name: info.name,
      volume: 1.0,
      enabled: true,
    }));
    mergedPath.value = null;
  }

  function toggleTrackEnabled(id: string) {
    const track = tracks.value.find((t) => t.id === id);
    if (track) {
      track.enabled = !track.enabled;
    }
  }

  async function addTrack() {
    try {
      const selected = await open({
        multiple: true,
        filters: [{ name: "Audio", extensions: AUDIO_EXTENSIONS }],
      });
      if (selected) {
        const paths = Array.isArray(selected) ? selected : [selected];
        for (const path of paths) {
          // Avoid duplicates
          if (!tracks.value.some((t) => t.path === path)) {
            tracks.value.push({
              id: crypto.randomUUID(),
              path,
              name: getFileName(path),
              volume: 1.0,
              enabled: true,
            });
          }
        }
        mergedPath.value = null;
      }
    } catch (e) {
      console.error("Failed to select file:", e);
    }
  }

  function removeTrack(id: string) {
    tracks.value = tracks.value.filter((t) => t.id !== id);
    mergedPath.value = null;
  }

  function updateTrackVolume(id: string, volume: number) {
    const track = tracks.value.find((t) => t.id === id);
    if (track) {
      track.volume = volume;
    }
  }

  function moveTrackUp(id: string) {
    const index = tracks.value.findIndex((t) => t.id === id);
    if (index > 0) {
      const temp = tracks.value[index];
      tracks.value[index] = tracks.value[index - 1];
      tracks.value[index - 1] = temp;
    }
  }

  function moveTrackDown(id: string) {
    const index = tracks.value.findIndex((t) => t.id === id);
    if (index < tracks.value.length - 1) {
      const temp = tracks.value[index];
      tracks.value[index] = tracks.value[index + 1];
      tracks.value[index + 1] = temp;
    }
  }

  async function mergeAllTracks() {
    if (!canMerge.value) return;

    isMerging.value = true;
    mergedPath.value = null;

    try {
      // Only merge enabled tracks
      const enabledTracks = tracks.value.filter(t => t.enabled);
      const trackInputs = enabledTracks.map((t) => ({
        path: t.path,
        volume: t.volume,
      }));

      const response = await invoke<{ output_path: string }>("merge_multi_tracks", {
        tracks: trackInputs,
      });

      mergedPath.value = response.output_path;
    } catch (e) {
      console.error("Failed to merge tracks:", e);
      alert(String(e));
    } finally {
      isMerging.value = false;
    }
  }

  // Merge specific tracks with volumes (for ResultView multi-track)
  async function mergeSelectedTracks(selectedTracks: { path: string; volume: number }[]) {
    if (selectedTracks.length < 1) return;

    isMerging.value = true;
    mergedPath.value = null;

    try {
      const response = await invoke<{ output_path: string }>("merge_multi_tracks", {
        tracks: selectedTracks,
      });

      mergedPath.value = response.output_path;
    } catch (e) {
      console.error("Failed to merge tracks:", e);
      alert(String(e));
    } finally {
      isMerging.value = false;
    }
  }

  // Legacy 2-track merge for Result/History views
  async function mergeTracks(bgmPath: string, vocalsPath: string) {
    isMerging.value = true;
    mergedPath.value = null;

    try {
      const response = await invoke<{ output_path: string }>("merge_tracks", {
        bgmPath,
        vocalsPath,
        bgmVolume: bgmVolume.value,
        vocalsVolume: vocalsVolume.value,
      });

      mergedPath.value = response.output_path;
    } catch (e) {
      console.error("Failed to merge tracks:", e);
      alert(String(e));
    } finally {
      isMerging.value = false;
    }
  }

  function resetMergeVolumes() {
    bgmVolume.value = 1.0;
    vocalsVolume.value = 1.0;
    mergedPath.value = null;
  }

  function clearAllTracks() {
    tracks.value = [];
    mergedPath.value = null;
  }

  function resetAllVolumes() {
    tracks.value.forEach((t) => (t.volume = 1.0));
    mergedPath.value = null;
  }

  return {
    // Multi-track
    tracks,
    canMerge,
    loadTracksFromResult,
    addTrack,
    removeTrack,
    updateTrackVolume,
    toggleTrackEnabled,
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
  };
}
