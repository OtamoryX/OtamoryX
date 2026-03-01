<template>
  <button
    :type="type"
    :disabled="disabled"
    :class="[
      'glass-button relative overflow-hidden font-medium transition-all duration-200',
      'focus:outline-none focus:ring-2 focus:ring-[var(--accent)]/40',
      'disabled:opacity-50 disabled:cursor-not-allowed',
      'active:scale-95',
      sizeClasses,
      variantClasses,
      radiusClasses,
      fullWidth && 'w-full',
      className,
    ]"
    @click="handleClick"
  >
    <div v-if="loading" class="flex items-center justify-center">
      <svg class="animate-spin -ml-1 mr-2 h-4 w-4" fill="none" viewBox="0 0 24 24">
        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
        <path class="opacity-75" fill="currentColor"
          d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
      </svg>
      <span>{{ loadingText || "加载中..." }}</span>
    </div>

    <div v-else class="flex items-center justify-center">
      <slot name="icon" />
      <span v-if="$slots.default || text">
        <slot>{{ text }}</slot>
      </span>
    </div>
  </button>
</template>

<script setup lang="ts">
import { computed } from "vue";
interface Props {
  type?: "button" | "submit" | "reset";
  variant?: "primary" | "secondary" | "success" | "danger" | "warning" | "ghost";
  size?: "xs" | "sm" | "md" | "lg" | "xl";
  radius?: "sm" | "md" | "lg" | "xl" | "full";
  disabled?: boolean;
  loading?: boolean;
  loadingText?: string;
  text?: string;
  fullWidth?: boolean;
  glowEffect?: boolean;
  className?: string;
}

const props = withDefaults(defineProps<Props>(), {
  type: "button",
  variant: "primary",
  size: "md",
  radius: "lg",
  disabled: false,
  loading: false,
  fullWidth: false,
  glowEffect: false,
  className: "",
});

const emit = defineEmits<{
  click: [];
}>();

const sizeClasses = computed(() => {
  const sizeMap = {
    xs: "px-2 py-1 text-xs",
    sm: "px-3 py-1.5 text-sm",
    md: "px-4 py-2 text-sm",
    lg: "px-6 py-3 text-base",
    xl: "px-8 py-4 text-lg",
  };
  return sizeMap[props.size];
});

const variantClasses = computed(() => {
  const variantMap = {
    primary: "text-white bg-[var(--accent)] border border-[var(--accent)] hover:bg-[var(--accent-hover)]",
    secondary: "text-[var(--text-primary)] bg-[var(--bg-tertiary)] border border-[var(--border)] hover:bg-[var(--border)]",
    success: "text-white bg-green-600 border border-green-600 hover:bg-green-700",
    danger: "text-white bg-red-600 border border-red-600 hover:bg-red-700",
    warning: "text-white bg-yellow-600 border border-yellow-600 hover:bg-yellow-700",
    ghost: "text-[var(--text-secondary)] bg-transparent border border-transparent hover:bg-[var(--bg-tertiary)] hover:text-[var(--text-primary)]",
  };
  return variantMap[props.variant];
});

const radiusClasses = computed(() => {
  const radiusMap = {
    sm: "rounded",
    md: "rounded-md",
    lg: "rounded-lg",
    xl: "rounded-xl",
    full: "rounded-full",
  };
  return radiusMap[props.radius];
});

const handleClick = () => {
  if (!props.disabled && !props.loading) {
    emit("click");
  }
};
</script>
