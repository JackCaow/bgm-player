import { computed, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { parseError, formatErrorMessage } from "@/utils/errorHandler";
import { AUDIO_EXTENSIONS, blobToWavBytes } from "@/utils/format";
import { normalizeSelectedPath } from "@/utils/path";
import { useNotifications } from "./useNotifications";

interface KokoroTtsResponse {
  output_path: string;
}

function isZipVoiceDialogVariant(variant: string): boolean {
  const v = (variant || "").trim().toLowerCase().replace(/_/g, "-");
  return v === "dialog" || v === "dialog-stereo";
}

function toDialogTaggedText(text: string): string {
  const raw = (text || "").trim();
  if (!raw) return raw;
  if (/\[(S1|S2)\]/i.test(raw)) return raw;
  return `[S1] ${raw}`;
}

export function useKokoroTts() {
  const { success, error: showError } = useNotifications();

  const text = ref("");
  const ttsEngine = ref("kokoro");
  const zipvoiceVariant = ref("distill");
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
  const refAudioPath2 = ref("");
  const refText2 = ref("");
  const isRecording = ref(false);
  const recordingTarget = ref<1 | 2 | null>(null);
  const recordingSeconds = ref(0);

  const isGenerating = ref(false);
  const isMerging = ref(false);
  const outputPath = ref<string | null>(null);
  const dryOutputPath = ref<string | null>(null);
  const isZipVoice = computed(() => ttsEngine.value === "zipvoice");
  const isZipVoiceDialog = computed(
    () => isZipVoice.value && isZipVoiceDialogVariant(zipvoiceVariant.value)
  );

  const canGenerate = computed(() => {
    if (!text.value.trim() || isGenerating.value) return false;
    if (isZipVoice.value) {
      if (!isZipVoiceDialog.value) {
        if (!refAudioPath.value.trim()) return false;
        if (!refText.value.trim()) return false;
      } else {
        const hasSecondAny = !!refAudioPath2.value.trim() || !!refText2.value.trim();
        if (hasSecondAny) {
          if (!refAudioPath.value.trim() || !refText.value.trim()) return false;
          if (!refAudioPath2.value.trim() || !refText2.value.trim()) return false;
        }
      }
      return true;
    }
    if (voice.value === "custom" && !refAudioPath.value.trim()) return false;
    return true;
  });
  const canMerge = computed(() =>
    !!dryOutputPath.value && !!bgmPath.value && !isGenerating.value && !isMerging.value
  );

  watch(voice, (v) => {
    if (v !== "custom") {
      refAudioPath.value = "";
      refText.value = "";
      refAudioPath2.value = "";
      refText2.value = "";
    }
  });

  watch(ttsEngine, (engine) => {
    const normalized = engine === "zipvoice" ? "zipvoice" : "kokoro";
    if (normalized !== engine) {
      ttsEngine.value = normalized;
      return;
    }
    if (normalized === "zipvoice") {
      voice.value = "custom";
      return;
    }
    refAudioPath2.value = "";
    refText2.value = "";
    if (voice.value === "custom" || voice.value === "default" || !voice.value.trim()) {
      voice.value = "af_heart";
    }
  });

  watch(zipvoiceVariant, (variant) => {
    const normalized = (() => {
      const v = (variant || "").trim().toLowerCase().replace(/_/g, "-");
      if (v === "basic" || v === "distill" || v === "dialog" || v === "dialog-stereo") return v;
      return "distill";
    })();
    if (normalized !== variant) {
      zipvoiceVariant.value = normalized;
      return;
    }
    if (!isZipVoiceDialogVariant(normalized)) {
      refAudioPath2.value = "";
      refText2.value = "";
    }
  }, { immediate: true });

  async function selectOutputDir() {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (selected) {
        outputDir.value = normalizeSelectedPath(selected as string);
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
        bgmPath.value = normalizeSelectedPath(selected as string);
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
        const path = normalizeSelectedPath(selected as string);
        refAudioPath.value = path;
      }
    } catch (e) {
      console.error("Failed to select ref audio:", e);
    }
  }

  async function selectRefAudio2() {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "Audio", extensions: AUDIO_EXTENSIONS }],
      });
      if (selected) {
        const path = normalizeSelectedPath(selected as string);
        refAudioPath2.value = path;
      }
    } catch (e) {
      console.error("Failed to select second ref audio:", e);
    }
  }

  let recordingTimer: ReturnType<typeof setInterval> | null = null;
  let mediaRecorder: MediaRecorder | null = null;
  let useBrowserRecording = false;

  async function startRecordingInternal(target: 1 | 2) {
    if (isRecording.value) return;
    recordingTarget.value = target;
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
            if (recordingTarget.value === 2) {
              refAudioPath2.value = path;
            } else {
              refAudioPath.value = path;
            }
            recordingTarget.value = null;
          } catch (e) {
            console.error("Failed to save recorded audio:", e);
            showError("录音保存失败", String(e), 5000);
            recordingTarget.value = null;
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
      recordingTarget.value = null;
    }
  }

  async function startRecording() {
    await startRecordingInternal(1);
  }

  async function startRecording2() {
    await startRecordingInternal(2);
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
      if (recordingTarget.value === 2) {
        refAudioPath2.value = path;
      } else {
        refAudioPath.value = path;
      }
      recordingTarget.value = null;
    } catch (e) {
      console.error("Failed to stop recording:", e);
      showError("录音停止失败", String(e), 5000);
      recordingTarget.value = null;
    }
  }

  function clear() {
    text.value = "";
    bgmPath.value = "";
    refAudioPath.value = "";
    refText.value = "";
    refAudioPath2.value = "";
    refText2.value = "";
    recordingTarget.value = null;
    outputPath.value = null;
    dryOutputPath.value = null;
  }

  async function generate() {
    if (!canGenerate.value) return;

    isGenerating.value = true;
    outputPath.value = null;
    dryOutputPath.value = null;

    try {
      if (isZipVoiceDialog.value) {
        const s1Text = refText.value.trim();
        const s2Text = refText2.value.trim();
        const s1Audio = refAudioPath.value.trim();
        const s2Audio = refAudioPath2.value.trim();
        const hasDoubleRef = !!s1Audio && !!s2Audio && !!s1Text && !!s2Text;
        if (hasDoubleRef) {
          if (s1Text === s2Text) {
            showError("参考文本不合理", "S1 和 S2 的参考文本不能完全相同，请填写各自音频的真实内容。", 10000);
            return;
          }
        }
      }
      const ttsText =
        isZipVoiceDialog.value && isZipVoice.value
          ? toDialogTaggedText(text.value)
          : text.value;
      const response = await invoke<KokoroTtsResponse>("synthesize_kokoro_tts", {
        text: ttsText,
        voice: voice.value,
        language: language.value,
        speed: speed.value,
        outputDir: outputDir.value || null,
        ttsEngine: ttsEngine.value,
        zipvoiceVariant: zipvoiceVariant.value,
        refAudioPath: refAudioPath.value || null,
        refText: refText.value || null,
        refAudioPath2: refAudioPath2.value || null,
        refText2: refText2.value || null,
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
      const outputPathHint = `${baseDir}/${ttsEngine.value}_tts_mix_${now}.wav`;
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
    zipvoiceVariant,
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
    refAudioPath2,
    refText2,
    isRecording,
    recordingTarget,
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
    selectRefAudio2,
    startRecording,
    startRecording2,
    stopRecording,
    clear,
    generate,
    mergePreview,
  };
}
