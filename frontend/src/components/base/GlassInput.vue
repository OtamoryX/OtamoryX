<template>
  <div class="glass-input-group">
    <!-- 标签 -->
    <label
      v-if="label"
      :for="inputId"
      class="block text-sm font-medium text-white/80 mb-2"
    >
      {{ label }}
      <span v-if="required"
class="text-red-400 ml-1">*</span>
    </label>

    <!-- 输入框容器 -->
    <div class="relative">
      <!-- 前置图标 -->
      <div
        v-if="$slots.prefix"
        class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none"
      >
        <div class="text-white/60">
          <slot name="prefix" />
        </div>
      </div>

      <!-- 输入框 -->
      <input
        :id="inputId"
        :type="type"
        :value="modelValue"
        :placeholder="placeholder"
        :disabled="disabled"
        :readonly="readonly"
        :maxlength="maxlength"
        :class="[
          'glass-input w-full transition-all duration-300',
          'bg-white/10 backdrop-blur-md border border-white/20',
          'text-white placeholder-white/50',
          'focus:outline-none focus:ring-2 focus:ring-blue-400/50 focus:border-blue-400/50',
          'hover:bg-white/15 hover:border-white/30',
          'disabled:opacity-50 disabled:cursor-not-allowed',
          sizeClasses,
          radiusClasses,
          $slots.prefix && 'pl-10',
          $slots.suffix && 'pr-10',
          error &&
            'border-red-400/50 focus:ring-red-400/50 focus:border-red-400/50',
          className,
        ]"
        @input="handleInput"
        @blur="handleBlur"
        @focus="handleFocus"
        @keyup.enter="handleEnter"
      />

      <!-- 后置图标 -->
      <div
        v-if="$slots.suffix"
        class="absolute inset-y-0 right-0 pr-3 flex items-center"
      >
        <div class="text-white/60">
          <slot name="suffix" />
        </div>
      </div>

      <!-- 清除按钮 -->
      <button
        v-if="clearable && modelValue && !disabled && !readonly"
        class="absolute inset-y-0 right-0 pr-3 flex items-center text-white/60 hover:text-white/80 transition-colors"
        type="button"
        @click="clearInput"
      >
        <svg
          class="w-4 h-4"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M6 18L18 6M6 6l12 12"
          />
        </svg>
      </button>
    </div>

    <!-- 帮助文本或错误信息 -->
    <div v-if="error || helperText"
class="mt-1 text-sm">
      <div v-if="error"
class="text-red-400">
        {{ error }}
      </div>
      <div v-else-if="helperText"
class="text-white/60">
        {{ helperText }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from "vue";
interface Props {
  modelValue?: string | number;
  type?: "text" | "password" | "email" | "number" | "tel" | "url" | "search";
  label?: string;
  placeholder?: string;
  disabled?: boolean;
  readonly?: boolean;
  required?: boolean;
  clearable?: boolean;
  maxlength?: number;
  size?: "sm" | "md" | "lg";
  radius?: "sm" | "md" | "lg" | "xl";
  error?: string;
  helperText?: string;
  className?: string;
}

const props = withDefaults(defineProps<Props>(), {
  type: "text",
  disabled: false,
  readonly: false,
  required: false,
  clearable: false,
  size: "md",
  radius: "lg",
  className: "",
});

const emit = defineEmits<{
  "update:modelValue": [value: string | number];
  blur: [event: Event];
  focus: [event: Event];
  enter: [event: Event];
  clear: [];
}>();

const inputId = ref(
  `glass-input-${Math.random().toString(36).substring(2, 9)}`,
);

const sizeClasses = computed(() => {
  const sizeMap = {
    sm: "px-3 py-2 text-sm",
    md: "px-4 py-2.5 text-base",
    lg: "px-5 py-3 text-lg",
  };
  return sizeMap[props.size];
});

const radiusClasses = computed(() => {
  const radiusMap = {
    sm: "rounded-md",
    md: "rounded-lg",
    lg: "rounded-xl",
    xl: "rounded-2xl",
  };
  return radiusMap[props.radius];
});

const handleInput = (event: Event) => {
  const target = event.target as HTMLInputElement;
  const value = props.type === "number" ? Number(target.value) : target.value;
  emit("update:modelValue", value);
};

const handleBlur = (event: Event) => {
  emit("blur", event);
};

const handleFocus = (event: Event) => {
  emit("focus", event);
};

const handleEnter = (event: Event) => {
  emit("enter", event);
};

const clearInput = () => {
  emit("update:modelValue", "");
  emit("clear");
};
</script>

<style scoped>
.glass-input {
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
}

.glass-input::placeholder {
  color: rgba(255, 255, 255, 0.5);
}

.glass-input:focus {
  background: rgba(255, 255, 255, 0.15);
}

/* 响应式调整 */
@media (max-width: 640px) {
  .glass-input {
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
  }
}
</style>
