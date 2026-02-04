import { ref } from "vue";

export interface ConfirmOptions {
  title: string;
  message: string;
  confirmText?: string;
  cancelText?: string;
  type?: "warning" | "danger" | "info";
}

interface ConfirmState extends ConfirmOptions {
  visible: boolean;
  resolve: ((value: boolean) => void) | null;
}

const state = ref<ConfirmState>({
  visible: false,
  title: "",
  message: "",
  confirmText: "确认",
  cancelText: "取消",
  type: "warning",
  resolve: null,
});

export function useConfirm() {
  function confirm(options: ConfirmOptions): Promise<boolean> {
    return new Promise((resolve) => {
      state.value = {
        ...options,
        visible: true,
        confirmText: options.confirmText || "确认",
        cancelText: options.cancelText || "取消",
        type: options.type || "warning",
        resolve,
      };
    });
  }

  function handleConfirm() {
    if (state.value.resolve) {
      state.value.resolve(true);
    }
    state.value.visible = false;
    state.value.resolve = null;
  }

  function handleCancel() {
    if (state.value.resolve) {
      state.value.resolve(false);
    }
    state.value.visible = false;
    state.value.resolve = null;
  }

  return {
    state,
    confirm,
    handleConfirm,
    handleCancel,
  };
}
