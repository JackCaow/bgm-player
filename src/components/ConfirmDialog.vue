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
        <AlertDialogCancel @click="handleCancel">
          {{ state.cancelText }}
        </AlertDialogCancel>
        <AlertDialogAction
          @click="handleConfirm"
          class="confirm-button"
          :style="{
            backgroundColor: state.type === 'danger' ? 'var(--error)' : state.type === 'warning' ? 'var(--warning)' : 'var(--primary)',
            color: '#fff'
          }"
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

