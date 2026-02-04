<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { useNotifications, type Notification } from "@/composables/useNotifications";

const { notifications, removeNotification } = useNotifications();

function getIcon(type: Notification["type"]): string {
  switch (type) {
    case "success":
      return "solar:check-circle-bold";
    case "error":
      return "solar:close-circle-bold";
    case "warning":
      return "solar:danger-triangle-bold";
    case "info":
      return "solar:info-circle-bold";
  }
}
</script>

<template>
  <div class="notifications-container">
    <TransitionGroup name="notification">
      <div
        v-for="notification in notifications"
        :key="notification.id"
        class="notification"
        :class="notification.type"
        @click="removeNotification(notification.id)"
      >
        <Icon :icon="getIcon(notification.type)" width="20" />
        <div class="notification-content">
          <div class="notification-title">{{ notification.title }}</div>
          <div v-if="notification.message" class="notification-message">
            {{ notification.message }}
          </div>
        </div>
        <button
          v-if="notification.action"
          class="notification-action"
          @click.stop="notification.action.onClick"
        >
          {{ notification.action.label }}
        </button>
        <button class="notification-close" @click.stop="removeNotification(notification.id)">
          <Icon icon="solar:close-square-linear" width="18" />
        </button>
      </div>
    </TransitionGroup>
  </div>
</template>

<style scoped>
.notifications-container {
  position: fixed;
  top: 16px;
  right: 16px;
  z-index: 2000;
  display: flex;
  flex-direction: column;
  gap: 12px;
  max-width: 400px;
}

.notification {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 16px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
  cursor: pointer;
  transition: all 0.2s;
}

.notification:hover {
  transform: translateY(-2px);
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.6);
}

.notification.success {
  border-left: 4px solid var(--success);
}

.notification.error {
  border-left: 4px solid var(--error);
}

.notification.warning {
  border-left: 4px solid var(--warning);
}

.notification.info {
  border-left: 4px solid var(--primary);
}

.notification-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.notification-title {
  font-weight: 600;
  font-size: 14px;
  color: var(--text);
}

.notification-message {
  font-size: 13px;
  color: var(--text-secondary);
  white-space: pre-line;
}

.notification-action {
  padding: 4px 12px;
  background: var(--primary);
  color: #000;
  border: none;
  border-radius: 4px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
}

.notification-action:hover {
  background: var(--primary-light);
}

.notification-close {
  padding: 4px;
  background: transparent;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.2s;
  display: flex;
  align-items: center;
  justify-content: center;
}

.notification-close:hover {
  color: var(--text);
}

/* Transition animations */
.notification-enter-active,
.notification-leave-active {
  transition: all 0.3s ease;
}

.notification-enter-from {
  opacity: 0;
  transform: translateX(100px);
}

.notification-leave-to {
  opacity: 0;
  transform: translateX(100px) scale(0.8);
}

.notification-move {
  transition: transform 0.3s ease;
}
</style>
