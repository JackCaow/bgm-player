<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { Icon } from "@iconify/vue";
import { useI18n } from "vue-i18n";
import { convertFileSrc } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import Slider from "@/components/ui/Slider.vue";
import { Checkbox } from "@/components/ui/checkbox";
import AudioPlayer from "@/components/AudioPlayer.vue";
import type { TrackInfo } from "@/types";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { formatPath } from "@/utils/format";
import { usePodcastAgent } from "@/composables/usePodcastAgent";

const { t } = useI18n();

const {
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
  selectOutputDir,
  selectBgm,
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
} = usePodcastAgent();

const ttsEngineOptions = [
  { value: "kokoro", label: "Kokoro" },
];

const kokoroVoiceOptions = [
  "af_alloy",
  "af_aoede",
  "af_bella",
  "af_heart",
  "af_jessica",
  "af_kore",
  "af_nicole",
  "af_nova",
  "af_river",
  "af_sarah",
  "af_sky",
  "am_adam",
  "am_echo",
  "am_eric",
  "am_fenrir",
  "am_liam",
  "am_michael",
  "am_onyx",
  "am_puck",
  "am_santa",
  "bf_alice",
  "bf_emma",
  "bf_isabella",
  "bf_lily",
  "bm_daniel",
  "bm_fable",
  "bm_george",
  "bm_lewis",
  "ef_dora",
  "em_alex",
  "em_santa",
  "ff_siwis",
  "hf_alpha",
  "hf_beta",
  "hm_omega",
  "hm_psi",
  "if_sara",
  "im_nicola",
  "jf_alpha",
  "jf_gongitsune",
  "jf_nezumi",
  "jf_tebukuro",
  "jm_kumo",
  "pf_dora",
  "pm_alex",
  "pm_santa",
  "zf_xiaobei",
  "zf_xiaoni",
  "zf_xiaoxiao",
  "zf_xiaoyi",
  "zm_yunjian",
  "zm_yunxi",
  "zm_yunxia",
  "zm_yunyang",
];

const voiceOptions = computed(() => kokoroVoiceOptions);

const languageOptions = [
  { value: "zh-cn", label: "中文" },
  { value: "en-us", label: "English" },
  { value: "ja", label: "日本語" },
];

const scriptText = computed(() => {
  if (lines.value.length === 0) return "";
  return lines.value
    .map((line, idx) => `${idx + 1}. ${line.role}: ${line.text}`)
    .join("\n");
});

async function copyScript() {
  if (!scriptText.value) return;
  try {
    await navigator.clipboard.writeText(scriptText.value);
  } catch (e) {
    console.error("Failed to copy script:", e);
  }
}

function estimateLineSeconds(text: string): number {
  const clean = text.trim();
  if (!clean) return 0;
  const isCjk = /[\u4e00-\u9fff]/.test(clean);
  const charsPerSecond = isCjk ? 4.2 : 11;
  const base = clean.length / charsPerSecond;
  const speedFactor = Math.max(0.6, Math.min(1.6, speed.value));
  return Math.max(0.4, base / speedFactor);
}

function formatSeconds(sec: number): string {
  return `${sec.toFixed(1)}s`;
}

const rawEstimatedTotalSeconds = computed(() => {
  if (lines.value.length === 0) return 0;
  const lineSum = lines.value.reduce((sum, l) => sum + estimateLineSeconds(l.text), 0);
  const pauseSum = Math.max(0, lines.value.length - 1) * (pauseMs.value / 1000);
  return lineSum + pauseSum;
});

const estimatedTotalSeconds = computed(() => {
  if (Number.isFinite(estimatedDurationSeconds.value as number)) {
    return Number(estimatedDurationSeconds.value);
  }
  return rawEstimatedTotalSeconds.value;
});

function formatDurationLabel(sec: number): string {
  if (!Number.isFinite(sec) || sec <= 0) return "0s";
  const s = Math.round(sec);
  const m = Math.floor(s / 60);
  const r = s % 60;
  return m > 0 ? `${m}m ${r}s` : `${r}s`;
}

function sectionState(idx: number): "done" | "active" | "pending" {
  if (!isGenerating.value || totalSections.value <= 0) return "pending";
  const current = activeSectionIndex.value - 1;
  if (idx < current) return "done";
  if (idx === current) return "active";
  return "pending";
}

function hasLineAudio(index: number): boolean {
  return !!lineAudioPaths.value[index];
}

const playingLineIndex = ref<number | null>(null);
let linePlayer: HTMLAudioElement | null = null;

function stopLinePlayback() {
  if (linePlayer) {
    linePlayer.pause();
    linePlayer.currentTime = 0;
    linePlayer = null;
  }
  playingLineIndex.value = null;
}

async function togglePlayLine(index: number) {
  const path = lineAudioPaths.value[index];
  if (!path) return;
  if (playingLineIndex.value === index) {
    stopLinePlayback();
    return;
  }
  stopLinePlayback();
  try {
    const player = new Audio(convertFileSrc(path));
    player.onended = () => {
      if (linePlayer === player) {
        stopLinePlayback();
      }
    };
    player.onerror = () => {
      if (linePlayer === player) {
        stopLinePlayback();
      }
    };
    linePlayer = player;
    playingLineIndex.value = index;
    await player.play();
  } catch (e) {
    console.error("Failed to play line audio:", e);
    stopLinePlayback();
  }
}

onBeforeUnmount(() => {
  stopLinePlayback();
});

watch(lineAudioPaths, () => {
  if (playingLineIndex.value === null) return;
  const idx = playingLineIndex.value;
  if (!lineAudioPaths.value[idx]) {
    stopLinePlayback();
  }
}, { deep: true });

const playbackTracks = computed<TrackInfo[]>(() => {
  if (!outputPath.value) return [];
  return [{ track_type: "vocals", path: outputPath.value, name: "Podcast" }];
});

const previewTracks = computed<TrackInfo[]>(() => {
  if (!dryOutputPath.value || !bgmPath.value) return [];
  return [
    { track_type: "vocals", path: dryOutputPath.value, name: "Podcast" },
    { track_type: "no_vocals", path: bgmPath.value, name: "BGM" },
  ];
});
</script>

<template>
  <div class="content-panel">
    <div class="panel-header">
      <h2>{{ t("podcast.title") }}</h2>
      <Button v-if="topic || outputPath" variant="outline" size="sm" @click="clear">
        {{ t("podcast.clear") }}
      </Button>
    </div>

    <div class="podcast-content">
      <section class="podcast-card">
        <div class="podcast-card-title">{{ t("podcast.script") }}</div>

        <label class="podcast-label">{{ t("podcast.topic") }}</label>
        <textarea
          v-model="topic"
          class="podcast-textarea"
          :placeholder="t('podcast.topicPlaceholder')"
          rows="4"
        />

        <div class="podcast-grid">
          <div>
            <label class="podcast-label">{{ t("podcast.engine") }}</label>
            <Select v-model:model-value="ttsEngine">
              <SelectTrigger class="w-full">
                <SelectValue :placeholder="t('podcast.engine')" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem v-for="item in ttsEngineOptions" :key="item.value" :value="item.value">
                  {{ item.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div>
            <label class="podcast-label">{{ t("podcast.style") }}</label>
            <input
              v-model="style"
              class="podcast-input"
              :placeholder="t('podcast.stylePlaceholder')"
            />
          </div>
          <div>
            <label class="podcast-label">{{ t("podcast.language") }}</label>
            <Select v-model:model-value="language">
              <SelectTrigger class="w-full">
                <SelectValue :placeholder="t('podcast.language')" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem v-for="item in languageOptions" :key="item.value" :value="item.value">
                  {{ item.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div>
            <label class="podcast-label">{{ t("podcast.pauseMs") }}</label>
            <input v-model.number="pauseMs" type="number" min="0" max="4000" class="podcast-input" />
          </div>
        </div>

        <div class="podcast-speed-wrap">
          <div class="podcast-speed-header">
            <label class="podcast-label">{{ t("podcast.speed") }}</label>
            <span class="podcast-speed-value">{{ speed.toFixed(2) }}x</span>
          </div>
          <Slider v-model="speed" :min="0.6" :max="1.6" :step="0.05" />
        </div>
      </section>

      <section class="podcast-card">
        <div class="podcast-card-title">{{ t("podcast.bgmTitle") }}</div>
        <div class="podcast-output" @click="selectBgm">
          <div class="podcast-output-left">
            <div class="podcast-label">{{ t("podcast.bgm") }}</div>
            <div class="podcast-output-path" :class="{ empty: !bgmPath }">
              {{ bgmPath ? formatPath(bgmPath) : t("podcast.selectBgm") }}
            </div>
          </div>
          <Icon icon="solar:alt-arrow-right-linear" width="16" class="podcast-output-arrow" />
        </div>

        <div class="podcast-speed-wrap">
          <div class="podcast-speed-header">
            <label class="podcast-label">{{ t("podcast.bgmVolume") }}</label>
            <span class="podcast-speed-value">{{ Math.round(bgmVolume * 100) }}%</span>
          </div>
          <Slider v-model="bgmVolume" :min="0" :max="2" :step="0.02" :disabled="!dryOutputPath" />
        </div>

        <div class="podcast-speed-wrap">
          <div class="podcast-speed-header">
            <label class="podcast-label">{{ t("podcast.podcastVolume") }}</label>
            <span class="podcast-speed-value">{{ Math.round(podcastVolume * 100) }}%</span>
          </div>
          <Slider v-model="podcastVolume" :min="0" :max="2" :step="0.02" :disabled="!dryOutputPath" />
        </div>

        <label class="podcast-loop-toggle">
          <Checkbox v-model="loopBgm" :disabled="!dryOutputPath" />
          <span>{{ t("podcast.loopBgm") }}: {{ loopBgm ? t("podcast.on") : t("podcast.off") }}</span>
        </label>

        <Button
          variant="outline"
          class="mt-3 h-10 w-full"
          :disabled="!canMerge"
          @click="mergeWithBgm"
        >
          <template v-if="!isMergingBgm">
            <Icon icon="solar:layers-bold" width="16" />
            {{ t("podcast.mixExport") }}
          </template>
          <template v-else>
            <Icon icon="solar:refresh-bold" width="16" class="animate-spin" />
            {{ t("podcast.mixing") }}
          </template>
        </Button>
      </section>

      <section class="podcast-card">
        <div class="podcast-card-title">{{ t("podcast.roles") }}</div>
        <div class="podcast-role-list">
          <div v-for="(role, idx) in roles" :key="idx" class="podcast-role-wrap">
            <div class="podcast-role-item">
              <input v-model="role.name" class="podcast-input" :placeholder="t('podcast.roleName')" />
              <Select v-model:model-value="role.voice">
                <SelectTrigger class="w-full">
                  <SelectValue :placeholder="t('podcast.roleVoice')" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem v-for="v in voiceOptions" :key="v" :value="v">
                    {{ v === 'custom' ? t('tts.customVoice') : v }}
                  </SelectItem>
                </SelectContent>
              </Select>
              <input
                v-model="role.persona"
                class="podcast-input"
                :placeholder="t('podcast.rolePersona')"
              />
              <Button variant="outline" size="sm" :disabled="roles.length <= 2" @click="removeRole(idx)">
                <Icon icon="solar:trash-bin-minimalistic-2-bold" width="16" />
              </Button>
            </div>
          </div>
        </div>
        <Button variant="outline" size="sm" class="mt-3" @click="addRole">
          <Icon icon="solar:add-circle-bold" width="16" />
          {{ t("podcast.addRole") }}
        </Button>
      </section>

      <section class="podcast-card">
        <div class="podcast-card-title">{{ t("podcast.llm") }}</div>
        <div class="podcast-grid">
          <div>
            <label class="podcast-label">{{ t("podcast.apiBase") }}</label>
            <input v-model="llmApiBase" class="podcast-input" placeholder="https://.../v1" />
          </div>
          <div>
            <label class="podcast-label">{{ t("podcast.model") }}</label>
            <input v-model="llmModel" class="podcast-input" placeholder="glm-4.7" />
          </div>
        </div>
        <label class="podcast-label mt-2">{{ t("podcast.apiKey") }}</label>
        <input v-model="llmApiKey" type="password" class="podcast-input" :placeholder="t('podcast.apiKeyHint')" />

        <div class="podcast-output" @click="selectOutputDir">
          <div class="podcast-output-left">
            <div class="podcast-label">{{ t("podcast.outputDir") }}</div>
            <div class="podcast-output-path" :class="{ empty: !outputDir }">
              {{ outputDir ? formatPath(outputDir) : t("podcast.outputDirOptional") }}
            </div>
          </div>
          <Icon icon="solar:alt-arrow-right-linear" width="16" class="podcast-output-arrow" />
        </div>
      </section>

      <section v-if="isGenerating" class="podcast-card">
        <div class="podcast-card-title">{{ t("podcast.generating") }}</div>
        <div class="mb-2 flex items-center justify-between gap-3">
          <span class="truncate text-sm text-foreground">{{ generationStatus || t("podcast.generating") }}</span>
          <span class="text-xs text-muted-foreground">{{ Math.round(generationProgress) }}%</span>
        </div>
        <div class="h-2 overflow-hidden rounded-full bg-muted">
          <div class="h-full rounded-full bg-primary transition-all duration-200" :style="{ width: `${generationProgress}%` }"></div>
        </div>
        <div v-if="activeSectionIndex > 0 && totalSections > 0" class="podcast-estimate mt-2">
          Section {{ activeSectionIndex }} / {{ totalSections }} · {{ generationStage }}
        </div>
      </section>

      <Button class="h-11 w-full rounded-xl" :disabled="!canGenerate" @click="generate">
        <template v-if="!isGenerating">
          <Icon icon="solar:document-text-bold" width="18" />
          1) 生成脚本预览
        </template>
        <template v-else>
          <Icon icon="solar:refresh-bold" width="18" class="animate-spin" />
          正在生成脚本...
        </template>
      </Button>

      <section v-if="lines.length > 0" class="podcast-card">
        <div class="podcast-card-title">2) 审阅并逐句生成语音</div>
        <div class="podcast-estimate">{{ t("podcast.estimatedTotal") }}: {{ formatSeconds(estimatedTotalSeconds) }}</div>
        <div v-if="targetDurationSeconds" class="podcast-estimate">
          目标时长: {{ formatDurationLabel(targetDurationSeconds) }}
        </div>
        <div class="podcast-actions">
          <Button variant="outline" :disabled="!canSynthesizeAll" @click="synthesizeAllLines">
            <template v-if="!isSynthesizingAll">
              <Icon icon="solar:play-bold" width="14" />
              3) 生成全部句子音频
            </template>
            <template v-else>
              <Icon icon="solar:refresh-bold" width="14" class="animate-spin" />
              正在逐句生成...
            </template>
          </Button>
          <Button variant="outline" :disabled="!canMergeLines" @click="mergeAllLines">
            <template v-if="!isMerging">
              <Icon icon="solar:layers-bold" width="14" />
              4) 合并全部音频
            </template>
            <template v-else>
              <Icon icon="solar:refresh-bold" width="14" class="animate-spin" />
              正在合并...
            </template>
          </Button>
        </div>
        <div class="podcast-lines">
          <div v-for="(line, idx) in lines" :key="idx" class="podcast-line-edit">
            <div class="podcast-line-edit-head">
              <span class="podcast-line-role">{{ idx + 1 }}. {{ line.role }}</span>
              <span class="podcast-line-time">{{ formatSeconds(estimateLineSeconds(line.text)) }}</span>
            </div>
            <div class="podcast-line-edit-grid">
              <Select :model-value="line.voice" @update:modelValue="updateLineVoice(idx, String($event))">
                <SelectTrigger class="w-full">
                  <SelectValue placeholder="Voice" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem v-for="v in voiceOptions" :key="v" :value="v">
                    {{ v === 'custom' ? t('tts.customVoice') : v }}
                  </SelectItem>
                </SelectContent>
              </Select>
              <textarea
                :value="line.text"
                class="podcast-textarea"
                rows="2"
                placeholder="编辑该句脚本后可单句重生"
                @input="updateLineText(idx, ($event.target as HTMLTextAreaElement).value)"
              />
              <div class="podcast-line-edit-actions">
                <Button
                  variant="outline"
                  size="sm"
                  :disabled="lineSynthesizingIndex !== null || isSynthesizingAll || !line.text.trim()"
                  @click="synthesizeLine(idx)"
                >
                  <template v-if="lineSynthesizingIndex !== idx">
                    <Icon icon="solar:microphone-3-bold" width="14" />
                    生成本句
                  </template>
                  <template v-else>
                    <Icon icon="solar:refresh-bold" width="14" class="animate-spin" />
                    生成中
                  </template>
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  :disabled="!hasLineAudio(idx)"
                  @click="togglePlayLine(idx)"
                >
                  <template v-if="playingLineIndex !== idx">
                    <Icon icon="solar:play-bold" width="14" />
                    播放本句
                  </template>
                  <template v-else>
                    <Icon icon="solar:stop-bold" width="14" />
                    停止
                  </template>
                </Button>
                <span class="podcast-line-status" :class="{ ready: hasLineAudio(idx) }">
                  {{ hasLineAudio(idx) ? "已生成" : "未生成" }}
                </span>
              </div>
            </div>
          </div>
        </div>
      </section>

      <section v-if="dryOutputPath" class="podcast-card">
        <div class="result-header">
          <Icon icon="solar:check-circle-bold" width="20" class="text-primary" />
          <span class="result-title">人声合并结果</span>
        </div>
        <div class="result-path">{{ dryOutputPath }}</div>
        <div v-if="scriptPath" class="result-path">{{ scriptPath }}</div>
        <AudioPlayer :tracks="[{ track_type: 'vocals', path: dryOutputPath, name: 'Podcast Voice' }]" />
      </section>

      <section v-if="previewTracks.length > 1" class="podcast-card">
        <div class="podcast-card-title">{{ t("podcast.previewTitle") }}</div>
        <AudioPlayer :tracks="previewTracks" />
      </section>

      <section v-if="outputPath && outputPath !== dryOutputPath" class="podcast-card">
        <div class="result-header">
          <Icon icon="solar:check-circle-bold" width="20" class="text-primary" />
          <span class="result-title">{{ t("podcast.result") }}</span>
        </div>
        <div class="result-path">{{ outputPath }}</div>
        <AudioPlayer :tracks="playbackTracks" />
      </section>

      <section v-if="outline.length > 0" class="podcast-card">
        <div class="podcast-card-title">{{ t("podcast.outlineTitle") }}</div>
        <div class="podcast-lines">
          <div v-for="(sec, idx) in outline" :key="idx" class="podcast-line">
            <span class="podcast-line-role" :class="`state-${sectionState(idx)}`">#{{ idx + 1 }}</span>
            <span class="podcast-line-text">
              <strong>{{ sec.title }}</strong> · {{ sec.objective }}
            </span>
            <span class="podcast-line-time">{{ formatDurationLabel(sec.target_seconds) }}</span>
          </div>
        </div>
      </section>

      <section class="podcast-card">
        <div class="podcast-script-header">
          <div class="podcast-card-title mb-0">{{ t("podcast.scriptText") }}</div>
          <Button variant="outline" size="sm" :disabled="!scriptText" @click="copyScript">
            <Icon icon="solar:copy-bold" width="14" />
            {{ t("podcast.copyScript") }}
          </Button>
        </div>
        <textarea
          class="podcast-textarea podcast-script-textarea"
          :value="scriptText"
          :placeholder="t('podcast.noScriptYet')"
          readonly
        />
      </section>
    </div>
  </div>
</template>

<style scoped>
.podcast-content {
  flex: 1;
  padding: 0 32px 32px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.podcast-card {
  border: 1px solid var(--border);
  background: var(--bg-card);
  border-radius: 14px;
  padding: 16px;
}

.podcast-card-title {
  margin-bottom: 10px;
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 0.3px;
  text-transform: uppercase;
  color: var(--text-secondary);
}

.podcast-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}

.podcast-label {
  display: block;
  margin-bottom: 6px;
  color: var(--text-secondary);
  font-size: 12px;
}

.podcast-input,
.podcast-textarea {
  width: 100%;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--bg);
  color: var(--text);
  padding: 10px 12px;
}

.podcast-textarea {
  resize: vertical;
  margin-bottom: 12px;
}

.podcast-speed-wrap {
  margin-top: 12px;
}

.podcast-speed-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.podcast-speed-value {
  font-size: 12px;
  color: var(--text-secondary);
}

.podcast-role-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.podcast-role-item {
  display: grid;
  grid-template-columns: 1fr 1fr 2fr auto;
  gap: 10px;
}

.podcast-output {
  margin-top: 12px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--bg);
  padding: 12px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  cursor: pointer;
}

.podcast-output:hover {
  background: var(--bg-hover);
}

.podcast-output-path {
  color: var(--text-secondary);
  font-size: 12px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.podcast-output-path.empty {
  color: var(--text-muted);
}

.podcast-lines {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.podcast-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  margin-bottom: 10px;
}

.podcast-line-edit {
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 10px;
  background: var(--bg);
}

.podcast-line-edit-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.podcast-line-edit-grid {
  display: grid;
  grid-template-columns: 180px 1fr auto;
  gap: 8px;
  align-items: start;
}

.podcast-line-edit-actions {
  display: flex;
  flex-direction: column;
  gap: 6px;
  align-items: flex-end;
}

.podcast-line-status {
  font-size: 12px;
  color: var(--text-secondary);
}

.podcast-line-status.ready {
  color: var(--success);
}

.podcast-line {
  display: flex;
  gap: 8px;
  font-size: 13px;
  align-items: flex-start;
}

.podcast-line-role {
  color: var(--primary);
  font-weight: 600;
  min-width: 72px;
}

.podcast-line-role.state-done {
  color: var(--success);
}

.podcast-line-role.state-active {
  color: var(--primary);
}

.podcast-line-role.state-pending {
  color: var(--text-secondary);
}

.podcast-line-text {
  color: var(--text);
  flex: 1;
}

.podcast-line-time {
  color: var(--text-secondary);
  font-size: 12px;
  min-width: 42px;
  text-align: right;
}

.podcast-estimate {
  margin-bottom: 8px;
  font-size: 12px;
  color: var(--text-secondary);
}

.podcast-script-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 8px;
}

.podcast-script-textarea {
  min-height: 220px;
  font-size: 13px;
  line-height: 1.55;
  white-space: pre-wrap;
}

.podcast-loop-toggle {
  margin-top: 12px;
  min-height: 40px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--bg);
  padding: 0 12px;
  display: inline-flex;
  align-items: center;
  gap: 10px;
  color: var(--text-secondary);
  font-size: 13px;
}

.result-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}

.result-title {
  font-weight: 600;
}

.result-path {
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 8px;
  word-break: break-all;
}

.podcast-role-wrap {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.podcast-role-ref {
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg);
}

.podcast-role-ref-actions {
  display: flex;
  gap: 8px;
}

.podcast-role-ref-preview {
  margin-top: 8px;
}

.podcast-role-ref-path {
  font-size: 12px;
  color: var(--text-secondary);
  display: block;
  margin-bottom: 4px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.podcast-role-ref-player {
  width: 100%;
  height: 32px;
}

.podcast-role-ref-text {
  width: 100%;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-card);
  color: var(--text);
  padding: 8px 10px;
  font-size: 13px;
}

.podcast-role-ref-hint {
  margin-top: 4px;
  font-size: 11px;
  color: var(--text-muted);
}

.mt-1 {
  margin-top: 4px;
}

@media (max-width: 1000px) {
  .podcast-grid {
    grid-template-columns: 1fr;
  }
  .podcast-role-item {
    grid-template-columns: 1fr;
  }
  .podcast-line-edit-grid {
    grid-template-columns: 1fr;
  }
  .podcast-line-edit-actions {
    align-items: stretch;
  }
}
</style>
