<script setup lang="ts">
import { ref, computed } from "vue";

const props = defineProps<{
  modelValue: number;
  min?: number;
  max?: number;
  step?: number;
  disabled?: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: number];
}>();

const sliderRef = ref<HTMLDivElement | null>(null);
const isDragging = ref(false);
const isHovering = ref(false);

const percentage = computed(() => {
  const range = (props.max || 100) - (props.min || 0);
  return ((props.modelValue - (props.min || 0)) / range) * 100;
});

function updateValue(clientX: number) {
  if (!sliderRef.value || props.disabled) return;

  const rect = sliderRef.value.getBoundingClientRect();
  const percent = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
  const range = (props.max || 100) - (props.min || 0);
  let value = (props.min || 0) + percent * range;

  // Apply step
  if (props.step) {
    value = Math.round(value / props.step) * props.step;
  }

  emit("update:modelValue", Math.max(props.min || 0, Math.min(props.max || 100, value)));
}

function handleMouseDown(event: MouseEvent) {
  if (props.disabled) return;
  isDragging.value = true;
  updateValue(event.clientX);
  document.addEventListener("mousemove", handleMouseMove);
  document.addEventListener("mouseup", handleMouseUp);
}

function handleMouseMove(event: MouseEvent) {
  if (isDragging.value) {
    updateValue(event.clientX);
  }
}

function handleMouseUp() {
  isDragging.value = false;
  document.removeEventListener("mousemove", handleMouseMove);
  document.removeEventListener("mouseup", handleMouseUp);
}

function handleClick(event: MouseEvent) {
  if (!isDragging.value) {
    updateValue(event.clientX);
  }
}
</script>

<template>
  <div
    ref="sliderRef"
    class="slider-root"
    :class="{ disabled, dragging: isDragging, hovering: isHovering }"
    @mousedown="handleMouseDown"
    @click="handleClick"
    @mouseenter="isHovering = true"
    @mouseleave="isHovering = false"
  >
    <div class="slider-track">
      <div class="slider-range" :style="{ width: `${percentage}%` }"></div>
    </div>
    <div class="slider-thumb" :style="{ left: `${percentage}%` }"></div>
  </div>
</template>

<style scoped>
.slider-root {
  position: relative;
  display: flex;
  width: 100%;
  touch-action: none;
  user-select: none;
  cursor: pointer;
  align-items: center;
  padding: 10px 0;
}

.slider-root.disabled {
  pointer-events: none;
  opacity: 0.5;
}

.slider-track {
  position: relative;
  height: 5px;
  width: 100%;
  flex-grow: 1;
  overflow: hidden;
  border-radius: 9999px;
  background: var(--bg-hover);
  transition: height 0.2s;
}

.slider-root.hovering .slider-track,
.slider-root.dragging .slider-track {
  height: 6px;
}

.slider-range {
  position: absolute;
  height: 100%;
  background: var(--primary);
  transition: width 0.1s ease-out;
  border-radius: 9999px;
}

.slider-thumb {
  position: absolute;
  display: block;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: var(--text);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4);
  transform: translateX(-50%) scale(0);
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  pointer-events: none;
  z-index: 10;
}

.slider-root.hovering .slider-thumb,
.slider-root.dragging .slider-thumb {
  transform: translateX(-50%) scale(1);
}

.slider-root.dragging .slider-thumb {
  transform: translateX(-50%) scale(1.2);
  box-shadow: 0 3px 12px rgba(0, 0, 0, 0.5), 0 0 0 6px color-mix(in srgb, var(--primary) 25%, transparent);
}

.slider-thumb::before {
  content: '';
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  width: 24px;
  height: 24px;
  background: color-mix(in srgb, var(--primary) 20%, transparent);
  border-radius: 50%;
  opacity: 0;
  transition: all 0.2s;
}

.slider-root.hovering .slider-thumb::before {
  opacity: 1;
}

.slider-root.dragging .slider-thumb::before {
  width: 32px;
  height: 32px;
  background: color-mix(in srgb, var(--primary) 30%, transparent);
}
</style>
