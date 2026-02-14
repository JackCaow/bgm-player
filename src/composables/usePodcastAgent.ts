import { computed, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { parseError, formatErrorMessage } from "@/utils/errorHandler";
import { AUDIO_EXTENSIONS, blobToWavBytes } from "@/utils/format";
import { normalizeSelectedPath } from "@/utils/path";
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
  zipvoiceVariant: string;
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

type TtsEngine = "kokoro" | "zipvoice";
type ZipVoiceVariant = "basic" | "distill" | "dialog" | "dialog-stereo";

const topic = ref("");
const ttsEngine = ref("kokoro");
const zipvoiceVariant = ref<ZipVoiceVariant>("distill");
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

function normalizeEngine(engine: string): TtsEngine {
  return (engine || "").trim().toLowerCase() === "zipvoice" ? "zipvoice" : "kokoro";
}

function normalizeZipVoiceVariant(variant: string): ZipVoiceVariant {
  const normalized = (variant || "").trim().toLowerCase().replace(/_/g, "-");
  if (normalized === "basic" || normalized === "distill" || normalized === "dialog" || normalized === "dialog-stereo") {
    return normalized;
  }
  return "distill";
}

function isDialogZipVoiceVariant(variant: string): boolean {
  const normalized = normalizeZipVoiceVariant(variant);
  return normalized === "dialog" || normalized === "dialog-stereo";
}

function hasDialogSpeakerTag(text: string): boolean {
  return /\[(S1|S2)\]/i.test(text);
}

function toDialogTaggedText(roleName: string, text: string): string {
  const raw = (text || "").trim();
  if (!raw) return raw;
  if (hasDialogSpeakerTag(raw)) return raw;

  const roleIndex = Math.max(0, roles.value.findIndex((r) => r.name === roleName));
  const speaker = roleIndex % 2 === 0 ? "S1" : "S2";
  return `[${speaker}] ${raw}`;
}

function buildDialogRoleSpeakerMap(dialogLines: PodcastLine[]): Map<string, "S1" | "S2"> {
  const map = new Map<string, "S1" | "S2">();
  let nextSpeaker: "S1" | "S2" = "S1";

  for (const line of dialogLines) {
    const roleName = (line.role || "").trim();
    if (!roleName || map.has(roleName)) continue;
    map.set(roleName, nextSpeaker);
    nextSpeaker = nextSpeaker === "S1" ? "S2" : "S1";
  }

  if (map.size === 0) {
    for (const role of roles.value) {
      const roleName = (role.name || "").trim();
      if (!roleName || map.has(roleName)) continue;
      map.set(roleName, nextSpeaker);
      nextSpeaker = nextSpeaker === "S1" ? "S2" : "S1";
      if (map.size >= 2) break;
    }
  }

  return map;
}

function buildDialogOneShotTextFromLines(
  dialogLines: PodcastLine[],
  roleSpeakerMap: Map<string, "S1" | "S2">
): string {
  return dialogLines
    .map((line, idx) => {
      const raw = (line.text || "").trim();
      if (!raw) return "";
      if (hasDialogSpeakerTag(raw)) return raw;
      const roleName = (line.role || "").trim();
      const speaker = roleName
        ? (roleSpeakerMap.get(roleName) || (idx % 2 === 0 ? "S1" : "S2"))
        : (idx % 2 === 0 ? "S1" : "S2");
      return `[${speaker}] ${raw}`;
    })
    .filter((line) => !!line.trim())
    .join("\n");
}

function resolveDialogPrimaryRefs(roleSpeakerMap: Map<string, "S1" | "S2">) {
  const s1Role = roles.value.find((role) => roleSpeakerMap.get((role.name || "").trim()) === "S1");
  const s2Role = roles.value.find((role) => roleSpeakerMap.get((role.name || "").trim()) === "S2");

  const s1Audio = (s1Role?.refAudioPath || "").trim();
  const s1Text = (s1Role?.refText || "").trim();
  const s2Audio = (s2Role?.refAudioPath || "").trim();
  const s2Text = (s2Role?.refText || "").trim();

  return {
    s1RoleName: s1Role?.name || "S1",
    s2RoleName: s2Role?.name || "S2",
    s1Audio,
    s1Text,
    s2Audio,
    s2Text,
  };
}

function defaultRoleVoiceByEngine(engine: string, roleIndex = 0): string {
  if (normalizeEngine(engine) === "zipvoice") return "custom";
  const defaults = ["af_bella", "am_adam", "bf_emma", "bm_george"];
  return defaults[roleIndex % defaults.length];
}

function isVoiceCompatible(engine: string, voice: string): boolean {
  const v = (voice || "").trim();
  if (!v) return false;
  if (normalizeEngine(engine) === "zipvoice") return v === "custom";
  return v !== "default" && v !== "custom";
}

function loadPodcastSettings() {
  if (podcastSettingsLoaded) return;
  podcastSettingsLoaded = true;
  try {
    const raw = localStorage.getItem(PODCAST_SETTINGS_KEY);
    if (!raw) return;
    const parsed = JSON.parse(raw) as Partial<PodcastAgentSettings>;

    style.value = typeof parsed.style === "string" ? parsed.style : style.value;
    ttsEngine.value = normalizeEngine(typeof parsed.ttsEngine === "string" ? parsed.ttsEngine : ttsEngine.value);
    zipvoiceVariant.value = normalizeZipVoiceVariant(
      typeof parsed.zipvoiceVariant === "string" ? parsed.zipvoiceVariant : zipvoiceVariant.value
    );
    if (style.value === LEGACY_DEFAULT_STYLE) {
      style.value = "Practical, insightful, and conversational with clear takeaways";
    }
    language.value = typeof parsed.language === "string" ? parsed.language : language.value;
    speed.value = Number.isFinite(parsed.speed) ? Math.min(1.6, Math.max(0.6, Number(parsed.speed))) : speed.value;
    pauseMs.value = Number.isFinite(parsed.pauseMs) ? Math.min(4000, Math.max(0, Number(parsed.pauseMs))) : pauseMs.value;
    outputDir.value = typeof parsed.outputDir === "string" ? normalizeSelectedPath(parsed.outputDir) : outputDir.value;
    bgmPath.value = typeof parsed.bgmPath === "string" ? normalizeSelectedPath(parsed.bgmPath) : bgmPath.value;
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
          refAudioPath: typeof r.refAudioPath === "string" ? normalizeSelectedPath(r.refAudioPath) : "",
          refText: typeof r.refText === "string" ? r.refText : "",
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
      zipvoiceVariant: zipvoiceVariant.value,
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
  [ttsEngine, zipvoiceVariant, style, language, speed, pauseMs, outputDir, bgmPath, bgmVolume, podcastVolume, loopBgm, llmApiBase, llmApiKey, llmModel],
  savePodcastSettings
);
watch(roles, savePodcastSettings, { deep: true });

watch(zipvoiceVariant, (variant) => {
  zipvoiceVariant.value = normalizeZipVoiceVariant(variant);
}, { immediate: true });

watch(ttsEngine, (engine) => {
  const normalized = normalizeEngine(engine);
  ttsEngine.value = normalized;

  roles.value = roles.value.map((role, index) => {
    const next = { ...role };
    if (!isVoiceCompatible(normalized, next.voice)) {
      next.voice = defaultRoleVoiceByEngine(normalized, index);
    }
    return next;
  });

  if (lines.value.length > 0) {
    const roleVoiceMap = new Map(roles.value.map((r) => [r.name, r.voice]));
    lines.value = lines.value.map((line) => {
      const mapped = roleVoiceMap.get(line.role);
      if (mapped) {
        return { ...line, voice: mapped };
      }
      if (!isVoiceCompatible(normalized, line.voice)) {
        return { ...line, voice: defaultRoleVoiceByEngine(normalized) };
      }
      return line;
    });
  }
}, { immediate: true });

// Sync role voice changes to corresponding lines
watch(
  () => roles.value.map((r) => ({ name: r.name, voice: r.voice })),
  (newRoles, oldRoles) => {
    if (!oldRoles || lines.value.length === 0) return;
    for (let i = 0; i < newRoles.length; i++) {
      const prev = oldRoles[i];
      const curr = newRoles[i];
      if (!prev || !curr) continue;
      if (prev.name === curr.name && prev.voice !== curr.voice) {
        for (const line of lines.value) {
          if (line.role === curr.name && line.voice === prev.voice) {
            line.voice = curr.voice;
          }
        }
      }
    }
  },
  { deep: true }
);

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
    return roles.value.every((r) => !!r.name.trim() && isVoiceCompatible(ttsEngine.value, r.voice));
  });

  const canSynthesizeAll = computed(() => {
    if (isGenerating.value || isSynthesizingAll.value || lineSynthesizingIndex.value !== null) return false;
    return lines.value.length > 0;
  });

  const isDialogOneShotMode = computed(() =>
    normalizeEngine(ttsEngine.value) === "zipvoice" && isDialogZipVoiceVariant(zipvoiceVariant.value)
  );

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
        const path = normalizeSelectedPath(selected as string);
        roles.value[index].refAudioPath = path;
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
    if (isDialogOneShotMode.value) {
      dryOutputPath.value = null;
      outputPath.value = null;
      return;
    }
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
    if (isDialogOneShotMode.value) {
      dryOutputPath.value = null;
      outputPath.value = null;
      return;
    }
    if (lineAudioPaths.value[index]) {
      lineAudioPaths.value[index] = "";
      dryOutputPath.value = null;
      outputPath.value = null;
    }
  }

  function getRoleByName(roleName: string): PodcastRole | undefined {
    return roles.value.find((r) => r.name === roleName);
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
    if (isDialogOneShotMode.value) {
      if (!silent) {
        showError("Dialog 模式不支持逐句生成", "请使用“一次生成完整对话”按钮", 6000);
      }
      return;
    }
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
    const role = getRoleByName(line.role);
    const refAudioPath = (role?.refAudioPath || "").trim();
    const refText = (role?.refText || "").trim();
    const lineText = isDialogZipVoiceVariant(zipvoiceVariant.value)
      ? toDialogTaggedText(line.role, line.text)
      : line.text;
    const zipvoiceNeedsRef = normalizeEngine(ttsEngine.value) === "zipvoice"
      && !isDialogZipVoiceVariant(zipvoiceVariant.value);
    if (zipvoiceNeedsRef) {
      if (!refAudioPath || !refText) {
        showError(`第 ${index + 1} 句缺少参考信息`, "ZipVoice 需要角色参考音频和参考文本", 8000);
        return;
      }
    }

    lineSynthesizingIndex.value = index;
    try {
      const response = await invoke<{ output_path: string }>("synthesize_kokoro_tts", {
        text: lineText,
        voice: line.voice,
        language: language.value,
        speed: speed.value,
        outputDir: outputDir.value || null,
        ttsEngine: ttsEngine.value,
        zipvoiceVariant: zipvoiceVariant.value,
        refAudioPath: refAudioPath || null,
        refText: refText || null,
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

    if (isDialogOneShotMode.value) {
      try {
        const validLines = lines.value.filter((line) => !!line.text.trim());
        if (validLines.length === 0) {
          showError("脚本为空", "请先填写至少一句对话", 6000);
          return;
        }

        const roleSpeakerMap = buildDialogRoleSpeakerMap(validLines);
        const dialogText = buildDialogOneShotTextFromLines(validLines, roleSpeakerMap);
        if (!dialogText.trim()) {
          showError("脚本为空", "请先填写至少一句对话", 6000);
          return;
        }
        const refs = resolveDialogPrimaryRefs(roleSpeakerMap);
        const s1Incomplete = (!!refs.s1Audio && !refs.s1Text) || (!refs.s1Audio && !!refs.s1Text);
        const s2Incomplete = (!!refs.s2Audio && !refs.s2Text) || (!refs.s2Audio && !!refs.s2Text);
        if (s1Incomplete) {
          showError(
            "S1 参考信息不完整",
            `角色 ${refs.s1RoleName} 的参考音频与参考文本需要同时填写或同时留空`,
            8000
          );
          return;
        }
        if (s2Incomplete) {
          showError(
            "S2 参考信息不完整",
            `角色 ${refs.s2RoleName} 的参考音频与参考文本需要同时填写或同时留空`,
            8000
          );
          return;
        }
        if (refs.s1Text && refs.s2Text) {
          if (refs.s1Text === refs.s2Text) {
            showError(
              "参考文本不合理",
              "S1 和 S2 的参考文本不能完全相同，请填写各自音频的真实内容。",
              10000
            );
            return;
          }
        }

        const response = await invoke<{ output_path: string }>("synthesize_kokoro_tts", {
          text: dialogText,
          voice: "custom",
          language: language.value,
          speed: speed.value,
          outputDir: outputDir.value || null,
          ttsEngine: ttsEngine.value,
          zipvoiceVariant: zipvoiceVariant.value,
          refAudioPath: refs.s1Audio || null,
          refText: refs.s1Text || null,
          refAudioPath2: refs.s2Audio || null,
          refText2: refs.s2Text || null,
        });

        dryOutputPath.value = response.output_path;
        outputPath.value = response.output_path;
        lineAudioPaths.value = [];
        success("Dialog 一次生成完成，可直接试听或继续混入 BGM", response.output_path, 5000);
      } catch (e) {
        const errorDetails = parseError(e);
        const errorMessage = formatErrorMessage(errorDetails);
        showError("Dialog 语音生成失败", errorMessage, 12000);
      } finally {
        isSynthesizingAll.value = false;
      }
      return;
    }

    // Build batch items for all non-empty lines
    const batchItems: { text: string; voice: string; language: string; speed: number; ref_audio_path?: string; ref_text?: string }[] = [];
    const batchIndexMap: number[] = []; // maps batch index -> line index
    for (let i = 0; i < lines.value.length; i++) {
      const line = lines.value[i];
      if (!line.text.trim()) continue;
      if (!isVoiceCompatible(ttsEngine.value, line.voice)) continue;
      const role = getRoleByName(line.role);
      const refAudioPath = (role?.refAudioPath || "").trim();
      const refText = (role?.refText || "").trim();
      const zipvoiceNeedsRef = normalizeEngine(ttsEngine.value) === "zipvoice"
        && !isDialogZipVoiceVariant(zipvoiceVariant.value);
      if (zipvoiceNeedsRef && (!refAudioPath || !refText)) {
        showError(
          "角色参考信息不完整",
          `第 ${i + 1} 句（${line.role}）缺少 ZipVoice 参考音频或参考文本`,
          10000
        );
        isSynthesizingAll.value = false;
        return;
      }
      batchItems.push({
        text: isDialogZipVoiceVariant(zipvoiceVariant.value)
          ? toDialogTaggedText(line.role, line.text)
          : line.text,
        voice: line.voice,
        language: language.value,
        speed: speed.value,
        ref_audio_path: refAudioPath || undefined,
        ref_text: refText || undefined,
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
        zipvoiceVariant: zipvoiceVariant.value,
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
    zipvoiceVariant,
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
    isDialogOneShotMode,
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
