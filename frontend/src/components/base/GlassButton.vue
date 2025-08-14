<template>
  <button
    :type="type"
    :disabled="disabled"
    :class="[
      'glass-button relative overflow-hidden font-medium transition-all duration-300',
      'bg-white/10 backdrop-blur-md border border-white/20',
      'hover:bg-white/15 hover:border-white/30',
      'focus:outline-none focus:ring-2 focus:ring-white/20 focus:ring-offset-2 focus:ring-offset-transparent',
      'disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:bg-white/10',
      'active:scale-95',
      sizeClasses,
      variantClasses,
      radiusClasses,
      fullWidth && 'w-full',
      className,
    ]"
    @click="handleClick"
  >
    <!-- 加载状态 -->
    <div v-if="loading"
class="flex items-center justify-center">
      <svg
        class="animate-spin -ml-1 mr-2 h-4 w-4"
        fill="none"
        viewBox="0 0 24 24"
      >
        <circle
          class="opacity-25"
          cx="12"
          cy="12"
          r="10"
          stroke="currentColor"
          stroke-width="4"
        />
        <path
          class="opacity-75"
          fill="currentColor"
          d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
        />
      </svg>
      <span>{{ loadingText || "加载中..." }}</span>
    </div>

    <!-- 正常内容 -->
    <div v-else
class="flex items-center justify-center">
      <!-- 图标插槽 -->
      <slot name="icon" />

      <!-- 文本内容 -->
      <span v-if="$slots.default || text">
        <slot>{{ text }}</slot>
      </span>
    </div>

    <!-- 光泽效果 -->
    <div v-if="glowEffect"
class="glass-shine absolute inset-0 opacity-20" />
  </button>
</template>

<script setup lang="ts">
import { computed } from "vue";
interface Props {
  type?: "button" | "submit" | "reset";
  variant?:
    | "primary"
    | "secondary"
    | "success"
    | "danger"
    | "warning"
    | "ghost";
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
    primary:
      "text-white bg-blue-600/80 border-blue-400/30 hover:bg-blue-600/90",
    secondary:
      "text-gray-300 bg-gray-600/80 border-gray-400/30 hover:bg-gray-600/90",
    success:
      "text-white bg-green-600/80 border-green-400/30 hover:bg-green-600/90",
    danger: "text-white bg-red-600/80 border-red-400/30 hover:bg-red-600/90",
    warning:
      "text-white bg-yellow-600/80 border-yellow-400/30 hover:bg-yellow-600/90",
    ghost: "text-white bg-white/5 border-white/10 hover:bg-white/10",
  };
  return variantMap[props.variant];
});

const radiusClasses = computed(() => {
  const radiusMap = {
    sm: "rounded-md",
    md: "rounded-lg",
    lg: "rounded-xl",
    xl: "rounded-2xl",
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

<style scoped>
.glass-button {
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
}

.glass-button::before {
  content: "";
  position: absolute;
  inset: 0;
  background: linear-gradient(
    135deg,
    rgba(255, 255, 255, 0.1) 0%,
    rgba(255, 255, 255, 0.05) 50%,
    rgba(255, 255, 255, 0.1) 100%
  );
  border-radius: inherit;
  pointer-events: none;
}

.glass-shine {
  background: linear-gradient(
    45deg,
    transparent 30%,
    rgba(255, 255, 255, 0.2) 50%,
    transparent 70%
  );
  transform: translateX(-100%) rotate(45deg);
  transition: transform 0.6s ease-in-out;
}

.glass-button:hover .glass-shine {
  transform: translateX(100%) rotate(45deg);
}

/* 响应式调整 */
@media (max-width: 640px) {
  .glass-button {
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
  }
}
</style>
