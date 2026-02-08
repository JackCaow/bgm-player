import { ref, computed, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type { ProgressPayload } from "@/types";
import { AUDIO_EXTENSIONS, VIDEO_EXTENSIONS } from "@/utils/format";
import { parseError, formatErrorMessage } from "@/utils/errorHandler";
import { useNotifications } from "./useNotifications";

export type VideoMixMode = "mix" | "bgmOnly";

export function useVideoMix() {
  const { success, error: showError } = useNotifications();

  const videoPath = ref<string>("");
  const bgmPath = ref<string>("");
  const outputDir = ref<string>("");

  const mode = ref<VideoMixMode>("mix");
  const bgmVolume = ref(1.0);
  const videoVolume = ref(1.0);
  const bgmOffsetSec = ref(0);
  const loopBgm = ref(true);
  const fadeInSec = ref(0);
  const fadeOutSec = ref(1);

  const isMixing = ref(false);
  const progress = ref(0);
  const status = ref("");
  const stage = ref("");
  const outputPath = ref<string | null>(null);

  const canStart = computed(() => {
    return !!videoPath.value && !!bgmPath.value && !isMixing.value;
  });

  let unlistenProgress: UnlistenFn | null = null;

  async function setupListeners() {
    unlistenProgress = await listen<ProgressPayload>("video-mix-progress", (event) => {
      if (!isMixing.value) return;
      progress.value = event.payload.progress;
      status.value = event.payload.status;
      stage.value = event.payload.stage;
    });
  }

  function cleanupListeners() {
    unlistenProgress?.();
  }

  onMounted(() => {
    setupListeners();
  });

  onUnmounted(() => {
    cleanupListeners();
  });

  async function selectVideo() {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "Video", extensions: VIDEO_EXTENSIONS }],
      });
      if (selected) {
        videoPath.value = selected as string;
        outputPath.value = null;
      }
    } catch (e) {
      console.error("Failed to select video:", e);
    }
  }

  async function selectBgm() {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "Audio", extensions: AUDIO_EXTENSIONS }],
      });
      if (selected) {
        bgmPath.value = selected as string;
        outputPath.value = null;
      }
    } catch (e) {
      console.error("Failed to select BGM:", e);
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

  function clear() {
    videoPath.value = "";
    bgmPath.value = "";
    outputPath.value = null;
    progress.value = 0;
    status.value = "";
    stage.value = "";
  }

  async function startMix() {
    if (!canStart.value) return;

    isMixing.value = true;
    outputPath.value = null;
    progress.value = 0;
    status.value = "开始处理...";
    stage.value = "init";

    try {
      const response = await invoke<{ output_path: string }>("mix_bgm_into_video", {
        videoPath: videoPath.value,
        bgmPath: bgmPath.value,
        mode: mode.value,
        bgmVolume: bgmVolume.value,
        videoVolume: videoVolume.value,
        bgmOffsetSec: bgmOffsetSec.value,
        loopBgm: loopBgm.value,
        fadeInSec: fadeInSec.value,
        fadeOutSec: fadeOutSec.value,
        outputDir: outputDir.value || null,
      });
      outputPath.value = response.output_path;
      success("导出完成", response.output_path, 4000);
    } catch (e) {
      const errorDetails = parseError(e);
      const errorMessage = formatErrorMessage(errorDetails);
      showError("处理失败", errorMessage, 10000);
    } finally {
      isMixing.value = false;
    }
  }

  return {
    videoPath,
    bgmPath,
    outputDir,
    mode,
    bgmVolume,
    videoVolume,
    bgmOffsetSec,
    loopBgm,
    fadeInSec,
    fadeOutSec,
    isMixing,
    progress,
    status,
    stage,
    outputPath,
    canStart,
    selectVideo,
    selectBgm,
    selectOutputDir,
    startMix,
    clear,
  };
}

