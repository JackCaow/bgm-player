<script setup lang="ts">
import { Button } from "@/components/ui";
</script>

<template>
  <div class="p-8 space-y-4">
    <h1 class="text-2xl font-bold">shadcn-vue Button Examples</h1>

    <div class="flex flex-wrap gap-4">
      <Button variant="default">Default Button</Button>
      <Button variant="secondary">Secondary</Button>
      <Button variant="outline">Outline</Button>
      <Button variant="ghost">Ghost</Button>
      <Button variant="destructive">Destructive</Button>
      <Button variant="link">Link</Button>
    </div>

    <div class="flex flex-wrap gap-4 items-center">
      <Button size="sm">Small</Button>
      <Button size="default">Default</Button>
      <Button size="lg">Large</Button>
      <Button size="icon">
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M5 12h14"/>
          <path d="m12 5 7 7-7 7"/>
        </svg>
      </Button>
    </div>
  </div>
</template>
