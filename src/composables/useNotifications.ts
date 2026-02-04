import { ref } from "vue";

export type NotificationType = "success" | "error" | "warning" | "info";

export interface Notification {
  id: string;
  type: NotificationType;
  title: string;
  message?: string;
  duration?: number;
  action?: {
    label: string;
    onClick: () => void;
  };
}

const notifications = ref<Notification[]>([]);

let notificationId = 0;

export function useNotifications() {
  function addNotification(notification: Omit<Notification, "id">) {
    const id = `notification-${++notificationId}`;
    const newNotification: Notification = {
      id,
      duration: 5000, // Default 5 seconds
      ...notification,
    };

    notifications.value.push(newNotification);

    // Auto remove after duration
    if (newNotification.duration && newNotification.duration > 0) {
      setTimeout(() => {
        removeNotification(id);
      }, newNotification.duration);
    }

    return id;
  }

  function removeNotification(id: string) {
    const index = notifications.value.findIndex((n) => n.id === id);
    if (index !== -1) {
      notifications.value.splice(index, 1);
    }
  }

  function clearAll() {
    notifications.value = [];
  }

  // Convenience methods
  function success(title: string, message?: string, duration?: number) {
    return addNotification({ type: "success", title, message, duration });
  }

  function error(title: string, message?: string, duration?: number) {
    return addNotification({ type: "error", title, message, duration: duration || 8000 });
  }

  function warning(title: string, message?: string, duration?: number) {
    return addNotification({ type: "warning", title, message, duration });
  }

  function info(title: string, message?: string, duration?: number) {
    return addNotification({ type: "info", title, message, duration });
  }

  return {
    notifications,
    addNotification,
    removeNotification,
    clearAll,
    success,
    error,
    warning,
    info,
  };
}
