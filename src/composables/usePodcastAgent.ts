import { computed, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { parseError, formatErrorMessage } from "@/utils/errorHandler";
import { AUDIO_EXTENSIONS, blobToWavBytes } from "@/utils/format";
import { useNotifications } from "./useNotifications";

export interface PodcastRole {
  name: string;
  voice: string;
  persona: string;
  refAudioPath?: string;
  refText?: string;
}

export interface PodcastLine {
  role: string;
  voice: string;
  text: string;
}

export interface PodcastOutlineSection {
  title: string;
  objective: string;
  target_seconds: number;
}

interface PodcastScriptPreviewResult {
  script_path: string;
  lines: PodcastLine[];
  outline?: PodcastOutlineSection[];
  estimated_duration_seconds?: number | null;
  target_duration_seconds?: number | null;
  style?: string;
}

interface PodcastProgressPayload {
  progress: number;
  status: string;
  stage: string;
}

interface PodcastAgentSettings {
  ttsEngine: string;
  style: string;
  language: string;
  speed: number;
  pauseMs: number;
  outputDir: string;
  bgmPath: string;
  bgmVolume: number;
  podcastVolume: number;
  loopBgm: boolean;
  llmApiBase: string;
  llmApiKey: string;
  llmModel: string;
  roles: PodcastRole[];
}

const PODCAST_SETTINGS_KEY = "podcast-agent-settings";
const LEGACY_DEFAULT_STYLE = "轻松但信息密度高，观点鲜明，有来有回";
const LEGACY_HOST_PERSONA = "主持人，控场、总结、抛问题";
const LEGACY_GUEST_PERSONA = "嘉宾，给经验、给案例、回应质疑";
const LEGACY_ADD_ROLE_PERSONA = "补充观点并推进对话";

const defaultRoles: PodcastRole[] = [
  { name: "Host", voice: "af_heart", persona: "Host who leads the flow, frames questions, and summarizes key points" },
  { name: "Guest", voice: "am_adam", persona: "Guest who provides practical experience, examples, and balanced counterpoints" },
];

const topic = ref("");
const ttsEngine = ref("kokoro");
const style = ref("Practical, insightful, and conversational with clear takeaways");
const language = ref("zh-cn");
const speed = ref(1.0);
const pauseMs = ref(220);
const outputDir = ref("");
const bgmPath = ref("");
const bgmVolume = ref(0.35);
const podcastVolume = ref(1.0);
const loopBgm = ref(true);

const llmApiBase = ref("");
const llmApiKey = ref("");
const llmModel = ref("glm-4.7");

const roles = ref<PodcastRole[]>([...defaultRoles]);

const isGenerating = ref(false);
const isSynthesizingAll = ref(false);
const isMerging = ref(false);
const isMergingBgm = ref(false);
const lineSynthesizingIndex = ref<number | null>(null);

const outputPath = ref<string | null>(null);
const dryOutputPath = ref<string | null>(null);
const scriptPath = ref<string | null>(null);
const lines = ref<PodcastLine[]>([]);
const lineAudioPaths = ref<string[]>([]);
const outline = ref<PodcastOutlineSection[]>([]);
const estimatedDurationSeconds = ref<number | null>(null);
const targetDurationSeconds = ref<number | null>(null);
const generationProgress = ref(0);
const generationStatus = ref("");
const generationStage = ref("");
const activeSectionIndex = ref(0);
const totalSections = ref(0);

let podcastSettingsLoaded = false;
let podcastProgressListenerReady = false;

function defaultRoleVoiceByEngine(_engine: string, _roleIndex = 0): string {
  return "af_bella";
}

function isVoiceCompatible(_engine: string, voice: string): boolean {
  const v = (voice || "").trim();
  return !!v;
}

function loadPodcastSettings() {
  if (podcastSettingsLoaded) return;
  podcastSettingsLoaded = true;
  try {
    const raw = localStorage.getItem(PODCAST_SETTINGS_KEY);
    if (!raw) return;
    const parsed = JSON.parse(raw) as Partial<PodcastAgentSettings>;

    style.value = typeof parsed.style === "string" ? parsed.style : style.value;
    ttsEngine.value = "kokoro";
    if (style.value === LEGACY_DEFAULT_STYLE) {
      style.value = "Practical, insightful, and conversational with clear takeaways";
    }
    language.value = typeof parsed.language === "string" ? parsed.language : language.value;
    speed.value = Number.isFinite(parsed.speed) ? Math.min(1.6, Math.max(0.6, Number(parsed.speed))) : speed.value;
    pauseMs.value = Number.isFinite(parsed.pauseMs) ? Math.min(4000, Math.max(0, Number(parsed.pauseMs))) : pauseMs.value;
    outputDir.value = typeof parsed.outputDir === "string" ? parsed.outputDir : outputDir.value;
    bgmPath.value = typeof parsed.bgmPath === "string" ? parsed.bgmPath : bgmPath.value;
    bgmVolume.value = Number.isFinite(parsed.bgmVolume) ? Math.min(2, Math.max(0, Number(parsed.bgmVolume))) : bgmVolume.value;
    podcastVolume.value = Number.isFinite(parsed.podcastVolume) ? Math.min(2, Math.max(0, Number(parsed.podcastVolume))) : podcastVolume.value;
    loopBgm.value = typeof parsed.loopBgm === "boolean" ? parsed.loopBgm : loopBgm.value;
    llmApiBase.value = typeof parsed.llmApiBase === "string" ? parsed.llmApiBase : llmApiBase.value;
    llmApiKey.value = typeof parsed.llmApiKey === "string" ? parsed.llmApiKey : llmApiKey.value;
    llmModel.value = typeof parsed.llmModel === "string" && parsed.llmModel ? parsed.llmModel : llmModel.value;

    if (Array.isArray(parsed.roles)) {
      const validRoles = parsed.roles
        .filter((r) => typeof r?.name === "string" && typeof r?.voice === "string")
        .map((r) => ({
          name: r.name,
          voice: r.voice,
          persona: typeof r.persona === "string" ? r.persona : "",
        }));
      if (validRoles.length >= 2) {
        validRoles.forEach((r) => {
          if (r.persona === LEGACY_HOST_PERSONA) {
            r.persona = "Host who leads the flow, frames questions, and summarizes key points";
          } else if (r.persona === LEGACY_GUEST_PERSONA) {
            r.persona = "Guest who provides practical experience, examples, and balanced counterpoints";
          } else if (r.persona === LEGACY_ADD_ROLE_PERSONA) {
            r.persona = "Adds complementary insights and moves the discussion forward";
          }
        });
        roles.value = validRoles;
      }
    }
  } catch (e) {
    console.error("Failed to load podcast settings:", e);
  }
}

function savePodcastSettings() {
  try {
    const payload: PodcastAgentSettings = {
      style: style.value,
      ttsEngine: ttsEngine.value,
      language: language.value,
      speed: speed.value,
      pauseMs: pauseMs.value,
      outputDir: outputDir.value,
      bgmPath: bgmPath.value,
      bgmVolume: bgmVolume.value,
      podcastVolume: podcastVolume.value,
      loopBgm: loopBgm.value,
      llmApiBase: llmApiBase.value,
      llmApiKey: llmApiKey.value,
      llmModel: llmModel.value,
      roles: roles.value,
    };
    localStorage.setItem(PODCAST_SETTINGS_KEY, JSON.stringify(payload));
  } catch (e) {
    console.error("Failed to save podcast settings:", e);
  }
}

loadPodcastSettings();
watch(
  [ttsEngine, style, language, speed, pauseMs, outputDir, bgmPath, bgmVolume, podcastVolume, loopBgm, llmApiBase, llmApiKey, llmModel],
  savePodcastSettings
);
watch(roles, savePodcastSettings, { deep: true });

export function usePodcastAgent() {
  const { success, error: showError } = useNotifications();

  if (!podcastProgressListenerReady) {
    podcastProgressListenerReady = true;
    listen<PodcastProgressPayload>("podcast-progress", (event) => {
      generationProgress.value = Number.isFinite(event.payload.progress)
        ? Math.max(0, Math.min(100, Number(event.payload.progress)))
        : generationProgress.value;
      generationStatus.value = event.payload.status || "";
      generationStage.value = event.payload.stage || "";

      const m = /^section:(\d+):(\d+)$/.exec(generationStage.value);
      if (m) {
        activeSectionIndex.value = Math.max(0, Number(m[1]));
        totalSections.value = Math.max(0, Number(m[2]));
      }
    }).catch((e) => {
      console.error("Failed to setup podcast progress listener:", e);
    });
  }

  const canGenerate = computed(() => {
    if (isGenerating.value || !topic.value.trim()) return false;
    if (roles.value.length < 2) return false;
    return roles.value.every((r) => r.name.trim() && isVoiceCompatible(ttsEngine.value, r.voice));
  });

  const canSynthesizeAll = computed(() => {
    if (isGenerating.value || isSynthesizingAll.value || lineSynthesizingIndex.value !== null) return false;
    return lines.value.length > 0;
  });

  const canMergeLines = computed(() => {
    if (isMerging.value || lines.value.length === 0) return false;
    return lineAudioPaths.value.length === lines.value.length && lineAudioPaths.value.every((p) => !!p);
  });

  const canMerge = computed(() =>
    !!dryOutputPath.value && !!bgmPath.value && !isGenerating.value && !isMergingBgm.value
  );

  function resetAudioOutputs() {
    outputPath.value = null;
    dryOutputPath.value = null;
    lineAudioPaths.value = lines.value.map(() => "");
  }

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
      }
    } catch (e) {
      console.error("Failed to select BGM:", e);
    }
  }

  function addRole() {
    roles.value.push({
      name: `Role ${roles.value.length + 1}`,
      voice: defaultRoleVoiceByEngine(ttsEngine.value, roles.value.length),
      persona: "Adds complementary insights and moves the discussion forward",
    });
  }

  function removeRole(index: number) {
    if (roles.value.length <= 2) return;
    roles.value.splice(index, 1);
  }

  async function selectRoleRefAudio(index: number) {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "Audio", extensions: AUDIO_EXTENSIONS }],
      });
      if (selected && roles.value[index]) {
        roles.value[index].refAudioPath = selected as string;
      }
    } catch (e) {
      console.error("Failed to select ref audio:", e);
    }
  }

  let roleRecordingTimer: ReturnType<typeof setInterval> | null = null;
  let roleMediaRecorder: MediaRecorder | null = null;
  let useRoleBrowserRecording = false;
  const roleRecordingIndex = ref<number | null>(null);
  const roleRecordingSeconds = ref(0);

  async function startRoleRecording(index: number) {
    // Try browser MediaRecorder first, fall back to ffmpeg
    if (navigator.mediaDevices?.getUserMedia) {
      try {
        const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
        const chunks: Blob[] = [];
        roleMediaRecorder = new MediaRecorder(stream);
        roleMediaRecorder.ondataavailable = (e) => {
          if (e.data.size > 0) chunks.push(e.data);
        };
        roleMediaRecorder.onstop = async () => {
          stream.getTracks().forEach((t) => t.stop());
          if (roleRecordingTimer) {
            clearInterval(roleRecordingTimer);
            roleRecordingTimer = null;
          }
          const targetIndex = roleRecordingIndex.value;
          roleRecordingIndex.value = null;
          roleRecordingSeconds.value = 0;
          const blob = new Blob(chunks, { type: roleMediaRecorder?.mimeType || "audio/webm" });
          const wavData = await blobToWavBytes(blob);
          const data = Array.from(wavData);
          try {
            const path = await invoke<string>("save_ref_audio", { audioData: data });
            if (targetIndex !== null && roles.value[targetIndex]) {
              roles.value[targetIndex].refAudioPath = path;
            }
          } catch (e) {
            console.error("Failed to save recorded audio:", e);
            showError("录音保存失败", String(e), 5000);
          }
        };
        roleMediaRecorder.start();
        useRoleBrowserRecording = true;
        roleRecordingIndex.value = index;
        roleRecordingSeconds.value = 0;
        roleRecordingTimer = setInterval(() => {
          roleRecordingSeconds.value++;
          if (roleRecordingSeconds.value >= 15) {
            stopRoleRecording();
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
      useRoleBrowserRecording = false;
      roleRecordingIndex.value = index;
      roleRecordingSeconds.value = 0;
      roleRecordingTimer = setInterval(() => {
        roleRecordingSeconds.value++;
        if (roleRecordingSeconds.value >= 15) {
          stopRoleRecording();
        }
      }, 1000);
    } catch (e) {
      console.error("Failed to start recording:", e);
      showError("录音启动失败", String(e), 5000);
    }
  }

  async function stopRoleRecording() {
    if (roleRecordingTimer) {
      clearInterval(roleRecordingTimer);
      roleRecordingTimer = null;
    }

    if (useRoleBrowserRecording && roleMediaRecorder && roleMediaRecorder.state !== "inactive") {
      roleMediaRecorder.stop(); // triggers onstop handler
      return;
    }

    const targetIndex = roleRecordingIndex.value;
    roleRecordingIndex.value = null;
    roleRecordingSeconds.value = 0;
    try {
      const path = await invoke<string>("stop_mic_recording", {});
      if (targetIndex !== null && roles.value[targetIndex]) {
        roles.value[targetIndex].refAudioPath = path;
      }
    } catch (e) {
      console.error("Failed to stop recording:", e);
      showError("录音停止失败", String(e), 5000);
    }
  }

  function clear() {
    isGenerating.value = false;
    isSynthesizingAll.value = false;
    isMerging.value = false;
    isMergingBgm.value = false;
    topic.value = "";
    outputPath.value = null;
    dryOutputPath.value = null;
    scriptPath.value = null;
    lines.value = [];
    lineAudioPaths.value = [];
    outline.value = [];
    estimatedDurationSeconds.value = null;
    targetDurationSeconds.value = null;
    generationProgress.value = 0;
    generationStatus.value = "";
    generationStage.value = "";
    activeSectionIndex.value = 0;
    totalSections.value = 0;
    lineSynthesizingIndex.value = null;
  }

  function updateLineText(index: number, value: string) {
    const line = lines.value[index];
    if (!line) return;
    line.text = value;
    if (lineAudioPaths.value[index]) {
      lineAudioPaths.value[index] = "";
      dryOutputPath.value = null;
      outputPath.value = null;
    }
  }

  function updateLineVoice(index: number, value: string) {
    const line = lines.value[index];
    if (!line) return;
    line.voice = value;
    if (lineAudioPaths.value[index]) {
      lineAudioPaths.value[index] = "";
      dryOutputPath.value = null;
      outputPath.value = null;
    }
  }

  async function generate() {
    if (!canGenerate.value) return;
    const invalidRoleIndex = roles.value.findIndex((r) => !isVoiceCompatible(ttsEngine.value, r.voice));
    if (invalidRoleIndex >= 0) {
      showError(
        "角色音色与模型不匹配",
        `第 ${invalidRoleIndex + 1} 个角色音色 ${roles.value[invalidRoleIndex].voice} 不支持当前模型 ${ttsEngine.value}`,
        8000
      );
      return;
    }
    isGenerating.value = true;
    outputPath.value = null;
    dryOutputPath.value = null;
    scriptPath.value = null;
    lines.value = [];
    lineAudioPaths.value = [];
    outline.value = [];
    estimatedDurationSeconds.value = null;
    targetDurationSeconds.value = null;
    generationProgress.value = 1;
    generationStatus.value = "Starting...";
    generationStage.value = "init";
    activeSectionIndex.value = 0;
    totalSections.value = 0;

    try {
      const response = await invoke<PodcastScriptPreviewResult>("generate_podcast_script_preview", {
        topic: topic.value,
        style: style.value,
        language: language.value,
        roles: roles.value,
        speed: speed.value,
        pauseMs: pauseMs.value,
        outputDir: outputDir.value || null,
        llmApiBase: llmApiBase.value || null,
        llmApiKey: llmApiKey.value || null,
        llmModel: llmModel.value || null,
      });

      scriptPath.value = response.script_path;
      lines.value = response.lines || [];
      lineAudioPaths.value = (response.lines || []).map(() => "");
      outline.value = response.outline || [];
      estimatedDurationSeconds.value = Number.isFinite(response.estimated_duration_seconds as number)
        ? Number(response.estimated_duration_seconds)
        : null;
      targetDurationSeconds.value = Number.isFinite(response.target_duration_seconds as number)
        ? Number(response.target_duration_seconds)
        : null;
      if (response.style) {
        style.value = response.style;
      }
      success("脚本生成完成，请先确认并编辑，再按句生成语音", response.script_path, 5000);
      generationProgress.value = 100;
      generationStatus.value = "Script ready";
      generationStage.value = "preview";
    } catch (e) {
      const errorDetails = parseError(e);
      const errorMessage = formatErrorMessage(errorDetails);
      showError("脚本生成失败", errorMessage, 12000);
      generationStatus.value = "Failed";
    } finally {
      isGenerating.value = false;
    }
  }

  async function synthesizeLine(index: number, silent = false) {
    if (index < 0 || index >= lines.value.length) return;
    const line = lines.value[index];
    if (!line.text.trim()) {
      if (!silent) {
        showError("第 " + (index + 1) + " 句为空", "请先填写文本", 5000);
      }
      return;
    }
    if (!isVoiceCompatible(ttsEngine.value, line.voice)) {
      showError(`第 ${index + 1} 句音色不匹配`, `当前模型 ${ttsEngine.value} 不支持音色 ${line.voice}`, 6000);
      return;
    }

    lineSynthesizingIndex.value = index;
    try {
      // Find the role's refAudioPath and refText if voice is "custom"
      const matchingRole = roles.value.find((r) => r.name === line.role);
      const refAudio = line.voice === "custom" && matchingRole?.refAudioPath
        ? matchingRole.refAudioPath
        : null;
      const refTextVal = line.voice === "custom" && matchingRole?.refText
        ? matchingRole.refText
        : null;

      const response = await invoke<{ output_path: string }>("synthesize_kokoro_tts", {
        text: line.text,
        voice: line.voice,
        language: language.value,
        speed: speed.value,
        outputDir: outputDir.value || null,
        ttsEngine: ttsEngine.value,
        refAudioPath: refAudio,
        refText: refTextVal,
      });

      lineAudioPaths.value[index] = response.output_path;
      dryOutputPath.value = null;
      outputPath.value = null;
      if (!silent) {
        success(`第 ${index + 1} 句生成完成`, response.output_path, 3500);
      }
    } catch (e) {
      const errorDetails = parseError(e);
      const errorMessage = formatErrorMessage(errorDetails);
      showError(`第 ${index + 1} 句生成失败`, errorMessage, 12000);
    } finally {
      lineSynthesizingIndex.value = null;
    }
  }

  async function synthesizeAllLines() {
    if (!canSynthesizeAll.value) return;
    isSynthesizingAll.value = true;
    dryOutputPath.value = null;
    outputPath.value = null;

    // Build batch items for all non-empty lines
    const batchItems: { text: string; voice: string; language: string; speed: number; ref_audio_path?: string; ref_text?: string }[] = [];
    const batchIndexMap: number[] = []; // maps batch index -> line index
    for (let i = 0; i < lines.value.length; i++) {
      const line = lines.value[i];
      if (!line.text.trim()) continue;
      if (!isVoiceCompatible(ttsEngine.value, line.voice)) continue;
      const matchingRole = roles.value.find((r) => r.name === line.role);
      const refAudio = line.voice === "custom" && matchingRole?.refAudioPath ? matchingRole.refAudioPath : undefined;
      const refTextVal = line.voice === "custom" && matchingRole?.refText ? matchingRole.refText : undefined;
      batchItems.push({
        text: line.text,
        voice: line.voice,
        language: language.value,
        speed: speed.value,
        ref_audio_path: refAudio,
        ref_text: refTextVal,
      });
      batchIndexMap.push(i);
    }

    if (batchItems.length === 0) {
      isSynthesizingAll.value = false;
      return;
    }

    try {
      const response = await invoke<{ output_paths: string[] }>("synthesize_tts_batch", {
        items: batchItems,
        ttsEngine: ttsEngine.value,
        outputDir: outputDir.value || null,
      });
      for (let j = 0; j < response.output_paths.length; j++) {
        const lineIdx = batchIndexMap[j];
        lineAudioPaths.value[lineIdx] = response.output_paths[j];
      }
      dryOutputPath.value = null;
      outputPath.value = null;
      success("全部句子语音生成完成，请手动点击\u201C合并全部音频\u201D", "", 4000);
    } catch (e) {
      const errorDetails = parseError(e);
      const errorMessage = formatErrorMessage(errorDetails);
      showError("批量语音生成失败", errorMessage, 12000);
    } finally {
      isSynthesizingAll.value = false;
    }
  }

  async function mergeAllLines() {
    if (!canMergeLines.value) return;
    isMerging.value = true;
    try {
      const response = await invoke<{ output_path: string }>("merge_audio_segments", {
        inputPaths: lineAudioPaths.value,
        outputDir: outputDir.value || null,
        outputPath: null,
      });
      dryOutputPath.value = response.output_path;
      outputPath.value = response.output_path;
      success("人声合并完成，可试听或继续混入 BGM", response.output_path, 5000);
    } catch (e) {
      const errorDetails = parseError(e);
      const errorMessage = formatErrorMessage(errorDetails);
      showError("合并人声失败", errorMessage, 12000);
    } finally {
      isMerging.value = false;
    }
  }

  async function mergeWithBgm() {
    if (!canMerge.value || !dryOutputPath.value) return;
    isMergingBgm.value = true;
    try {
      const now = Date.now();
      const baseDir = outputDir.value || dryOutputPath.value.split(/[/\\]/).slice(0, -1).join("/");
      const outputPathHint = `${baseDir}/podcast_mix_${now}.wav`;
      const response = await invoke<{ output_path: string }>("merge_tracks", {
        bgmPath: bgmPath.value,
        vocalsPath: dryOutputPath.value,
        bgmVolume: bgmVolume.value,
        vocalsVolume: podcastVolume.value,
        outputPath: outputPathHint,
        baseOnVocals: true,
        loopBgm: loopBgm.value,
      });
      outputPath.value = response.output_path;
      success("Podcast+BGM 合成完成", response.output_path, 5000);
    } catch (e) {
      const errorDetails = parseError(e);
      const errorMessage = formatErrorMessage(errorDetails);
      showError("Podcast+BGM 合成失败", errorMessage, 12000);
    } finally {
      isMergingBgm.value = false;
    }
  }

  return {
    topic,
    ttsEngine,
    style,
    language,
    speed,
    pauseMs,
    outputDir,
    bgmPath,
    bgmVolume,
    podcastVolume,
    loopBgm,
    llmApiBase,
    llmApiKey,
    llmModel,
    roles,
    isGenerating,
    isSynthesizingAll,
    lineSynthesizingIndex,
    isMerging,
    isMergingBgm,
    outputPath,
    dryOutputPath,
    scriptPath,
    lines,
    lineAudioPaths,
    outline,
    estimatedDurationSeconds,
    targetDurationSeconds,
    generationProgress,
    generationStatus,
    generationStage,
    activeSectionIndex,
    totalSections,
    canGenerate,
    canSynthesizeAll,
    canMergeLines,
    canMerge,
    roleRecordingIndex,
    roleRecordingSeconds,
    selectOutputDir,
    selectBgm,
    selectRoleRefAudio,
    startRoleRecording,
    stopRoleRecording,
    addRole,
    removeRole,
    clear,
    updateLineText,
    updateLineVoice,
    generate,
    synthesizeLine,
    synthesizeAllLines,
    mergeAllLines,
    mergeWithBgm,
    resetAudioOutputs,
  };
}
