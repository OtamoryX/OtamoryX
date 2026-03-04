<template>
  <BaseModal
    :show="show"
    :title="title"
    width="md"
    :z-index="9999"
    @close="$emit('close')"
  >
    <!-- 确认内容 -->
    <div class="space-y-4">
      <!-- 图标 -->
      <div v-if="showIcon" class="flex justify-center">
        <div :class="iconClasses">
          <svg
            class="w-6 h-6"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              v-if="type === 'danger'"
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z"
            />
            <path
              v-else-if="type === 'warning'"
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
            />
            <path
              v-else-if="type === 'info'"
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
            />
            <path
              v-else
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
            />
          </svg>
        </div>
      </div>

      <!-- 消息内容 -->
      <div class="text-center">
        <p class="text-[var(--text-primary)] leading-6">
          {{ message }}
        </p>
      </div>
    </div>

    <!-- 操作按钮 -->
    <template #footer>
      <div class="flex justify-end space-x-3">
        <GlassButton
          v-if="showCancel"
          variant="ghost"
          @click="$emit('close')"
        >
          {{ cancelText }}
        </GlassButton>
        <GlassButton
          :variant="confirmButtonVariant"
          :loading="loading"
          :loading-text="loadingText"
          @click="handleConfirm"
        >
          {{ confirmText }}
        </GlassButton>
      </div>
    </template>
  </BaseModal>
</template>

<script setup lang="ts">
import { computed } from "vue";
import BaseModal from "@/components/base/BaseModal.vue";
import GlassButton from "@/components/base/GlassButton.vue";

interface Props {
  show: boolean;
  title?: string;
  message: string;
  type?: "default" | "danger" | "warning" | "info";
  confirmText?: string;
  cancelText?: string;
  loadingText?: string;
  showIcon?: boolean;
  loading?: boolean;
  showCancel?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  title: "确认操作",
  type: "default",
  confirmText: "确认",
  cancelText: "取消",
  loadingText: "处理中...",
  showIcon: true,
  loading: false,
  showCancel: true,
});

const emit = defineEmits<{
  close: [];
  confirm: [];
}>();

const iconClasses = computed(() => {
  const baseClasses =
    "mx-auto shrink-0 flex items-center justify-center h-12 w-12 rounded-full backdrop-blur-md border";

  const typeMap = {
    danger: "bg-red-500/20 text-red-300 border-red-400/30",
    warning: "bg-yellow-500/20 text-yellow-300 border-yellow-400/30",
    info: "bg-blue-500/20 text-blue-300 border-blue-400/30",
    default: "bg-[var(--bg-tertiary)] text-[var(--text-secondary)] border-[var(--border)]",
  };

  return `${baseClasses} ${typeMap[props.type]}`;
});

const confirmButtonVariant = computed(() => {
  const typeMap = {
    danger: "danger",
    warning: "warning",
    info: "primary",
    default: "primary",
  } as const;

  return typeMap[props.type];
});

const handleConfirm = () => {
  if (!props.loading) {
    emit("confirm");
  }
};
</script>

<style scoped>
/* 样式已在Tailwind中定义，无需额外样式 */
</style>
