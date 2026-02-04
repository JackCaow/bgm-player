import { ref, computed } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { TrackInfo, TrackType } from "@/types";

// Global singleton state - persists across component mounts/unmounts
const audioTracks = ref<Map<TrackType, HTMLAudioElement>>(new Map());
const trackVolumes = ref<Map<TrackType, number>>(new Map());
const selectedTracks = ref<TrackType[]>([]);
const currentTracks = ref<TrackInfo[]>([]);

const isPlaying = ref(false);
const currentTime = ref(0);
const duration = ref(0);
const isVisible = ref(false);

let animationFrameId: number | null = null;

export function useGlobalPlayer() {
  const activeTracks = computed(() => new Set(selectedTracks.value));

  const progress = computed(() => {
    return duration.value > 0 ? (currentTime.value / duration.value) * 100 : 0;
  });

  const formattedCurrentTime = computed(() => formatTime(currentTime.value));
  const formattedDuration = computed(() => formatTime(duration.value));

  function formatTime(seconds: number): string {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, "0")}`;
  }

  function updateTimeDisplay() {
    const firstAudio = Array.from(audioTracks.value.values())[0];
    if (firstAudio && !firstAudio.paused) {
      currentTime.value = firstAudio.currentTime;
      animationFrameId = requestAnimationFrame(updateTimeDisplay);
    }
  }

  async function loadTracks(tracks: TrackInfo[]) {
    // Check if same tracks are already loaded
    const isSameTracks = currentTracks.value.length === tracks.length &&
      tracks.every((t, i) => currentTracks.value[i]?.path === t.path);

    if (isSameTracks && audioTracks.value.size > 0) {
      return;
    }

    stop();
    audioTracks.value.clear();
    trackVolumes.value.clear();
    currentTracks.value = tracks;

    try {
      const loadPromises: Promise<void>[] = [];

      for (const track of tracks) {
        const src = convertFileSrc(track.path);
        const audio = new Audio(src);

        audioTracks.value.set(track.track_type as TrackType, audio);
        trackVolumes.value.set(track.track_type as TrackType, 1.0);
        audio.volume = 1.0;

        loadPromises.push(
          new Promise((resolve) => {
            audio.addEventListener("loadedmetadata", () => resolve(), { once: true });
          })
        );
      }

      await Promise.all(loadPromises);

      const firstAudio = Array.from(audioTracks.value.values())[0];
      if (firstAudio) {
        duration.value = firstAudio.duration;
      }

      const syncPlayback = () => {
        const audios = Array.from(audioTracks.value.values());
        if (audios.length < 2) return;

        const referenceTime = audios[0].currentTime;
        for (let i = 1; i < audios.length; i++) {
          const diff = Math.abs(audios[i].currentTime - referenceTime);
          if (diff > 0.1) {
            audios[i].currentTime = referenceTime;
          }
        }
      };

      if (firstAudio) {
        firstAudio.addEventListener("timeupdate", syncPlayback);
        firstAudio.addEventListener("ended", () => {
          isPlaying.value = false;
          currentTime.value = 0;
          if (animationFrameId) {
            cancelAnimationFrame(animationFrameId);
          }
        });
      }

    } catch (error) {
      console.error("Failed to load audio tracks:", error);
      throw error;
    }
  }

  function play() {
    if (audioTracks.value.size === 0) return;

    if (selectedTracks.value.length === 0) {
      selectedTracks.value = Array.from(audioTracks.value.keys());
    }

    audioTracks.value.forEach((audio, type) => {
      if (selectedTracks.value.includes(type)) {
        const vol = trackVolumes.value.get(type) || 1.0;
        audio.volume = Math.min(1, vol);
        audio.play();
      } else {
        audio.volume = 0;
        audio.play();
      }
    });

    isPlaying.value = true;
    updateTimeDisplay();
  }

  function pause() {
    audioTracks.value.forEach((audio) => {
      audio.pause();
    });
    isPlaying.value = false;

    if (animationFrameId) {
      cancelAnimationFrame(animationFrameId);
    }
  }

  function stop() {
    pause();
    currentTime.value = 0;
    selectedTracks.value = [];

    audioTracks.value.forEach((audio) => {
      audio.currentTime = 0;
    });
  }

  function seek(time: number) {
    if (audioTracks.value.size === 0) return;

    const newTime = Math.max(0, Math.min(time, duration.value));
    audioTracks.value.forEach((audio) => {
      audio.currentTime = newTime;
    });
    currentTime.value = newTime;
  }

  function seekToPercent(percent: number) {
    const time = (percent / 100) * duration.value;
    seek(time);
  }

  function setTrackVolume(trackType: TrackType, volume: number) {
    const clampedVolume = Math.max(0, Math.min(2, volume));
    trackVolumes.value.set(trackType, clampedVolume);

    const audio = audioTracks.value.get(trackType);
    if (audio && selectedTracks.value.includes(trackType)) {
      audio.volume = Math.min(1, clampedVolume);
    }
  }

  function getTrackVolume(trackType: TrackType): number {
    return trackVolumes.value.get(trackType) || 1.0;
  }

  function toggleTrack(trackType: TrackType) {
    const index = selectedTracks.value.indexOf(trackType);
    if (index >= 0) {
      selectedTracks.value = selectedTracks.value.filter(t => t !== trackType);
      const audio = audioTracks.value.get(trackType);
      if (audio) {
        audio.volume = 0;
      }
    } else {
      selectedTracks.value = [...selectedTracks.value, trackType];
      const audio = audioTracks.value.get(trackType);
      if (audio && isPlaying.value) {
        const vol = trackVolumes.value.get(trackType) || 1.0;
        audio.volume = Math.min(1, vol);
      }
    }
  }

  function togglePlayPause() {
    if (isPlaying.value) {
      pause();
    } else {
      play();
    }
  }

  function hidePlayer() {
    isVisible.value = false;
  }

  return {
    // State
    isPlaying,
    currentTime,
    duration,
    progress,
    formattedCurrentTime,
    formattedDuration,
    audioTracks,
    trackVolumes,
    activeTracks,
    currentTracks,
    isVisible,
    // Actions
    loadTracks,
    play,
    pause,
    stop,
    seek,
    seekToPercent,
    setTrackVolume,
    getTrackVolume,
    toggleTrack,
    togglePlayPause,
    hidePlayer,
  };
}
