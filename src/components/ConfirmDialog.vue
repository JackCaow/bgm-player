<script setup lang="ts">
import { useConfirm } from "@/composables/useConfirm";
import {
  AlertDialog,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogAction,
  AlertDialogCancel,
} from "@/components/ui/alert-dialog";

const { state, handleConfirm, handleCancel } = useConfirm();
</script>

<template>
  <AlertDialog :open="state.visible">
    <AlertDialogContent class="confirm-content !border-0">
      <AlertDialogHeader>
        <AlertDialogTitle>{{ state.title }}</AlertDialogTitle>
        <AlertDialogDescription>
          {{ state.message }}
        </AlertDialogDescription>
      </AlertDialogHeader>
      <AlertDialogFooter>
        <AlertDialogCancel
          @click="handleCancel"
          class="border-[color:var(--primary)] text-[color:var(--primary)] hover:bg-[color:color-mix(in_srgb,var(--primary)_10%,transparent)] hover:text-[color:var(--primary)]"
        >
          {{ state.cancelText }}
        </AlertDialogCancel>
        <AlertDialogAction
          @click="handleConfirm"
          class="bg-[color:var(--primary)] text-black hover:bg-[color:color-mix(in_srgb,var(--primary)_88%,black)]"
        >
          {{ state.confirmText }}
        </AlertDialogAction>
      </AlertDialogFooter>
    </AlertDialogContent>
  </AlertDialog>
</template>

<style scoped>
:deep(.confirm-content) {
  border: none !important;
}
</style>
