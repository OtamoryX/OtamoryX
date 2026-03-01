<template>
  <div class="glass-input-group">
    <label v-if="label" :for="inputId" class="block text-sm font-medium text-[var(--text-primary)] mb-1.5">
      {{ label }}
      <span v-if="required" class="text-red-500 ml-0.5">*</span>
    </label>

    <div class="relative">
      <div v-if="$slots.prefix" class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
        <div class="text-[var(--text-tertiary)]">
          <slot name="prefix" />
        </div>
      </div>

      <input
        :id="inputId"
        :type="type"
        :value="modelValue"
        :placeholder="placeholder"
        :disabled="disabled"
        :readonly="readonly"
        :maxlength="maxlength"
        :class="[
          'glass-input w-full transition-all duration-200',
          'bg-[var(--bg-tertiary)] border border-[var(--border)]',
          'text-[var(--text-primary)] placeholder-[var(--text-tertiary)]',
          'focus:outline-none focus:ring-2 focus:ring-[var(--accent)]/40 focus:border-[var(--accent)]',
          'hover:border-[var(--text-tertiary)]',
          'disabled:opacity-50 disabled:cursor-not-allowed',
          sizeClasses,
          radiusClasses,
          $slots.prefix && 'pl-10',
          $slots.suffix && 'pr-10',
          error && 'border-red-500 focus:ring-red-500/40 focus:border-red-500',
          className,
        ]"
        @input="handleInput"
        @blur="handleBlur"
        @focus="handleFocus"
        @keyup.enter="handleEnter"
      />

      <div v-if="$slots.suffix" class="absolute inset-y-0 right-0 pr-3 flex items-center">
        <div class="text-[var(--text-tertiary)]">
          <slot name="suffix" />
        </div>
      </div>

      <button
        v-if="clearable && modelValue && !disabled && !readonly"
        class="absolute inset-y-0 right-0 pr-3 flex items-center text-[var(--text-tertiary)] hover:text-[var(--text-primary)] transition-colors"
        type="button"
        @click="clearInput"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>

    <div v-if="error || helperText" class="mt-1 text-sm">
      <div v-if="error" class="text-red-500">{{ error }}</div>
      <div v-else-if="helperText" class="text-[var(--text-tertiary)]">{{ helperText }}</div>
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

const inputId = ref(`glass-input-${Math.random().toString(36).substring(2, 9)}`);

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
    sm: "rounded",
    md: "rounded-md",
    lg: "rounded-lg",
    xl: "rounded-xl",
  };
  return radiusMap[props.radius];
});

const handleInput = (event: Event) => {
  const target = event.target as HTMLInputElement;
  const value = props.type === "number" ? Number(target.value) : target.value;
  emit("update:modelValue", value);
};

const handleBlur = (event: Event) => { emit("blur", event); };
const handleFocus = (event: Event) => { emit("focus", event); };
const handleEnter = (event: Event) => { emit("enter", event); };
const clearInput = () => { emit("update:modelValue", ""); emit("clear"); };
</script>
