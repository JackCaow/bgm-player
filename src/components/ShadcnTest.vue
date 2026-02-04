<script setup lang="ts">
import { Button } from "@/components/ui";
import { ref } from "vue";

const count = ref(0);
const message = ref("");

const handleClick = (variant: string) => {
  count.value++;
  message.value = `你点击了 ${variant} 按钮！点击次数: ${count.value}`;
};
</script>

<template>
  <div class="p-8 space-y-6 max-w-4xl mx-auto">
    <div class="space-y-2">
      <h1 class="text-3xl font-bold">shadcn-vue 测试页面</h1>
      <p class="text-gray-600">测试 Tailwind CSS v4 和 shadcn-vue 组件集成</p>
    </div>

    <div v-if="message" class="p-4 bg-green-100 border border-green-400 rounded-lg">
      <p class="text-green-800 font-medium">{{ message }}</p>
    </div>

    <div class="space-y-4">
      <h2 class="text-xl font-semibold">按钮变体 (Variants)</h2>
      <div class="flex flex-wrap gap-3">
        <Button variant="default" @click="handleClick('default')">
          Default Button
        </Button>
        <Button variant="secondary" @click="handleClick('secondary')">
          Secondary
        </Button>
        <Button variant="outline" @click="handleClick('outline')">
          Outline
        </Button>
        <Button variant="ghost" @click="handleClick('ghost')">
          Ghost
        </Button>
        <Button variant="destructive" @click="handleClick('destructive')">
          Destructive
        </Button>
        <Button variant="link" @click="handleClick('link')">
          Link
        </Button>
      </div>
    </div>

    <div class="space-y-4">
      <h2 class="text-xl font-semibold">按钮尺寸 (Sizes)</h2>
      <div class="flex flex-wrap items-center gap-3">
        <Button size="sm" @click="handleClick('small')">
          Small
        </Button>
        <Button size="default" @click="handleClick('default size')">
          Default
        </Button>
        <Button size="lg" @click="handleClick('large')">
          Large
        </Button>
        <Button size="icon" @click="handleClick('icon')">
          <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M5 12h14"/>
            <path d="m12 5 7 7-7 7"/>
          </svg>
        </Button>
      </div>
    </div>

    <div class="space-y-4">
      <h2 class="text-xl font-semibold">Tailwind CSS v4 工具类测试</h2>
      <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div class="p-4 bg-blue-100 rounded-lg border-2 border-blue-300">
          <p class="font-semibold text-blue-800">蓝色卡片</p>
          <p class="text-sm text-blue-600">Tailwind 颜色工具类</p>
        </div>
        <div class="p-4 bg-purple-100 rounded-lg border-2 border-purple-300">
          <p class="font-semibold text-purple-800">紫色卡片</p>
          <p class="text-sm text-purple-600">响应式布局</p>
        </div>
        <div class="p-4 bg-green-100 rounded-lg border-2 border-green-300">
          <p class="font-semibold text-green-800">绿色卡片</p>
          <p class="text-sm text-green-600">现代化设计</p>
        </div>
      </div>
    </div>

    <div class="mt-8 p-6 bg-gray-100 rounded-lg">
      <h3 class="text-lg font-semibold mb-2">✅ 集成成功！</h3>
      <ul class="space-y-1 text-sm text-gray-700">
        <li>✓ Tailwind CSS v4 正常工作</li>
        <li>✓ shadcn-vue Button 组件可用</li>
        <li>✓ 响应式设计生效</li>
        <li>✓ 自定义样式正常</li>
      </ul>
    </div>
  </div>
</template>
