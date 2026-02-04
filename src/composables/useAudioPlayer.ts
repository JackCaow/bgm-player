import { ref, computed, onUnmounted } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { AudioPlayerState, TrackInfo, TrackType } from "@/types";

export function useAudioPlayer() {
  const audioTracks = ref<Map<TrackType, HTMLAudioElement>>(new Map());
  const trackVolumes = ref<Map<TrackType, number>>(new Map());
  // Use array instead of Set for better Vue reactivity
  const selectedTracks = ref<TrackType[]>([]);

  const isPlaying = ref(false);
  const currentTime = ref(0);
  const duration = ref(0);

  let animationFrameId: number | null = null;

  // Computed set for easy lookup
  const activeTracks = computed(() => new Set(selectedTracks.value));

  const playerState = computed<AudioPlayerState>(() => ({
    isPlaying: isPlaying.value,
    currentTime: currentTime.value,
    duration: duration.value,
    volume: 1.0,
    activeTrack: selectedTracks.value.length === 0 ? null :
                 selectedTracks.value.length === audioTracks.value.size ? "all" :
                 selectedTracks.value[0],
  }));

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
    stop();
    audioTracks.value.clear();
    trackVolumes.value.clear();

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

    // If no tracks selected, select all
    if (selectedTracks.value.length === 0) {
      selectedTracks.value = Array.from(audioTracks.value.keys());
    }

    audioTracks.value.forEach((audio, type) => {
      if (selectedTracks.value.includes(type)) {
        const vol = trackVolumes.value.get(type) || 1.0;
        audio.volume = Math.min(1, vol);  // HTML5 Audio max is 1
        audio.play();
      } else {
        audio.volume = 0;
        audio.play(); // Play all but mute unselected for sync
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
      // HTML5 Audio volume is 0-1, so clamp to 1 max
      audio.volume = Math.min(1, clampedVolume);
    }
  }

  function getTrackVolume(trackType: TrackType): number {
    return trackVolumes.value.get(trackType) || 1.0;
  }

  function toggleTrack(trackType: TrackType) {
    console.log("[toggleTrack] called with:", trackType);
    console.log("[toggleTrack] before:", [...selectedTracks.value]);

    const index = selectedTracks.value.indexOf(trackType);
    if (index >= 0) {
      // Remove from selection
      selectedTracks.value = selectedTracks.value.filter(t => t !== trackType);
      // Mute if playing
      const audio = audioTracks.value.get(trackType);
      if (audio) {
        audio.volume = 0;
      }
    } else {
      // Add to selection
      selectedTracks.value = [...selectedTracks.value, trackType];
      // Unmute if playing
      const audio = audioTracks.value.get(trackType);
      if (audio && isPlaying.value) {
        const vol = trackVolumes.value.get(trackType) || 1.0;
        audio.volume = Math.min(1, vol);  // HTML5 Audio max is 1
      }
    }

    console.log("[toggleTrack] after:", [...selectedTracks.value]);
  }

  function togglePlayPause() {
    if (isPlaying.value) {
      pause();
    } else {
      play();
    }
  }

  function cleanup() {
    stop();

    if (animationFrameId) {
      cancelAnimationFrame(animationFrameId);
    }

    audioTracks.value.clear();
    trackVolumes.value.clear();
    duration.value = 0;
  }

  onUnmounted(() => {
    cleanup();
  });

  return {
    playerState,
    isPlaying,
    currentTime,
    duration,
    progress,
    formattedCurrentTime,
    formattedDuration,
    audioTracks,
    trackVolumes,
    activeTracks,
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
    cleanup,
  };
}
