import { computed, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { parseError, formatErrorMessage } from "@/utils/errorHandler";
import { AUDIO_EXTENSIONS, blobToWavBytes } from "@/utils/format";
import { useNotifications } from "./useNotifications";

interface KokoroTtsResponse {
  output_path: string;
}

export function useKokoroTts() {
  const { success, error: showError } = useNotifications();

  const text = ref("");
  const ttsEngine = ref("kokoro");
  const voice = ref("af_heart");
  const language = ref("en-us");
  const speed = ref(1.0);
  const outputDir = ref("");
  const bgmPath = ref("");
  const bgmVolume = ref(0.4);
  const ttsVolume = ref(1.0);
  const loopBgm = ref(true);
  const refAudioPath = ref("");
  const refText = ref("");
  const isRecording = ref(false);
  const recordingSeconds = ref(0);

  const isGenerating = ref(false);
  const isMerging = ref(false);
  const outputPath = ref<string | null>(null);
  const dryOutputPath = ref<string | null>(null);

  const canGenerate = computed(() => text.value.trim().length > 0 && !isGenerating.value);
  const canMerge = computed(() =>
    !!dryOutputPath.value && !!bgmPath.value && !isGenerating.value && !isMerging.value
  );

  watch(voice, (v) => {
    if (v !== "custom") {
      refAudioPath.value = "";
    }
  });

  async function selectOutputDir() {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (selected) {
        outputDir.value = selected as string;
      }
    } catch (e) {
      console.error("Failed to select output directory:", e);
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

  async function selectRefAudio() {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "Audio", extensions: AUDIO_EXTENSIONS }],
      });
      if (selected) {
        refAudioPath.value = selected as string;
      }
    } catch (e) {
      console.error("Failed to select ref audio:", e);
    }
  }

  let recordingTimer: ReturnType<typeof setInterval> | null = null;
  let mediaRecorder: MediaRecorder | null = null;
  let useBrowserRecording = false;

  async function startRecording() {
    // Try browser MediaRecorder first (better quality), fall back to ffmpeg
    if (navigator.mediaDevices?.getUserMedia) {
      try {
        const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
        const chunks: Blob[] = [];
        mediaRecorder = new MediaRecorder(stream);
        mediaRecorder.ondataavailable = (e) => {
          if (e.data.size > 0) chunks.push(e.data);
        };
        mediaRecorder.onstop = async () => {
          stream.getTracks().forEach((t) => t.stop());
          if (recordingTimer) {
            clearInterval(recordingTimer);
            recordingTimer = null;
          }
          isRecording.value = false;
          recordingSeconds.value = 0;
          const blob = new Blob(chunks, { type: mediaRecorder?.mimeType || "audio/webm" });
          const wavData = await blobToWavBytes(blob);
          const data = Array.from(wavData);
          try {
            const path = await invoke<string>("save_ref_audio", { audioData: data });
            refAudioPath.value = path;
          } catch (e) {
            console.error("Failed to save recorded audio:", e);
            showError("录音保存失败", String(e), 5000);
          }
        };
        mediaRecorder.start();
        useBrowserRecording = true;
        isRecording.value = true;
        recordingSeconds.value = 0;
        recordingTimer = setInterval(() => {
          recordingSeconds.value++;
          if (recordingSeconds.value >= 15) {
            stopRecording();
          }
        }, 1000);
        return;
      } catch (e) {
        console.warn("Browser recording unavailable, falling back to ffmpeg:", e);
      }
    }

    // Fallback: ffmpeg recording via Rust
    try {
      await invoke("start_mic_recording", { maxSeconds: 15 });
      useBrowserRecording = false;
      isRecording.value = true;
      recordingSeconds.value = 0;
      recordingTimer = setInterval(() => {
        recordingSeconds.value++;
        if (recordingSeconds.value >= 15) {
          stopRecording();
        }
      }, 1000);
    } catch (e) {
      console.error("Failed to start recording:", e);
      showError("录音启动失败", String(e), 5000);
    }
  }

  async function stopRecording() {
    if (recordingTimer) {
      clearInterval(recordingTimer);
      recordingTimer = null;
    }

    if (useBrowserRecording && mediaRecorder && mediaRecorder.state !== "inactive") {
      mediaRecorder.stop(); // triggers onstop handler which sets isRecording=false
      return;
    }

    isRecording.value = false;
    recordingSeconds.value = 0;
    try {
      const path = await invoke<string>("stop_mic_recording", {});
      refAudioPath.value = path;
    } catch (e) {
      console.error("Failed to stop recording:", e);
      showError("录音停止失败", String(e), 5000);
    }
  }

  function clear() {
    text.value = "";
    bgmPath.value = "";
    refAudioPath.value = "";
    refText.value = "";
    outputPath.value = null;
    dryOutputPath.value = null;
  }

  async function generate() {
    if (!canGenerate.value) return;

    isGenerating.value = true;
    outputPath.value = null;
    dryOutputPath.value = null;

    try {
      const response = await invoke<KokoroTtsResponse>("synthesize_kokoro_tts", {
        text: text.value,
        voice: voice.value,
        language: language.value,
        speed: speed.value,
        outputDir: outputDir.value || null,
        ttsEngine: ttsEngine.value,
        refAudioPath: refAudioPath.value || null,
        refText: refText.value || null,
      });

      outputPath.value = response.output_path;
      dryOutputPath.value = response.output_path;
      success("语音生成完成", response.output_path, 4500);
    } catch (e) {
      const errorDetails = parseError(e);
      const errorMessage = formatErrorMessage(errorDetails);
      showError("语音生成失败", errorMessage, 10000);
    } finally {
      isGenerating.value = false;
    }
  }

  async function mergePreview() {
    if (!canMerge.value || !dryOutputPath.value) return;

    isMerging.value = true;
    try {
      const now = Date.now();
      const baseDir = outputDir.value || dryOutputPath.value.split(/[/\\]/).slice(0, -1).join("/");
      const outputPathHint = `${baseDir}/kokoro_tts_mix_${now}.wav`;
      const response = await invoke<KokoroTtsResponse>("merge_tracks", {
        bgmPath: bgmPath.value,
        vocalsPath: dryOutputPath.value,
        bgmVolume: bgmVolume.value,
        vocalsVolume: ttsVolume.value,
        outputPath: outputPathHint,
        baseOnVocals: true,
        loopBgm: loopBgm.value,
      });
      outputPath.value = response.output_path;
      success("合成完成", response.output_path, 4500);
    } catch (e) {
      const errorDetails = parseError(e);
      const errorMessage = formatErrorMessage(errorDetails);
      showError("合成失败", errorMessage, 10000);
    } finally {
      isMerging.value = false;
    }
  }

  return {
    text,
    ttsEngine,
    voice,
    language,
    speed,
    outputDir,
    bgmPath,
    bgmVolume,
    ttsVolume,
    loopBgm,
    refAudioPath,
    refText,
    isRecording,
    recordingSeconds,
    isGenerating,
    isMerging,
    outputPath,
    dryOutputPath,
    canGenerate,
    canMerge,
    selectOutputDir,
    selectBgm,
    selectRefAudio,
    startRecording,
    stopRecording,
    clear,
    generate,
    mergePreview,
  };
}
