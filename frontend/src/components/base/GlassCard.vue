<template>
  <div
    :class="[
      'glass-card relative overflow-hidden transition-all duration-200',
      'bg-[var(--bg-card)] border border-[var(--border)]',
      sizeClasses,
      radiusClasses,
      clickable && 'cursor-pointer hover:border-[var(--accent)]',
      className,
    ]"
    @click="handleClick"
  >
    <slot />
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
interface Props {
  size?: "sm" | "md" | "lg" | "xl" | "full";
  radius?: "sm" | "md" | "lg" | "xl" | "2xl";
  clickable?: boolean;
  shineEffect?: boolean;
  glowEffect?: boolean;
  bottomBorder?: boolean;
  className?: string;
}

const props = withDefaults(defineProps<Props>(), {
  size: "md",
  radius: "lg",
  clickable: false,
  shineEffect: false,
  glowEffect: false,
  bottomBorder: false,
  className: "",
});

const emit = defineEmits<{
  click: [];
}>();

const sizeClasses = computed(() => {
  const sizeMap = {
    sm: "p-4",
    md: "p-6",
    lg: "p-8",
    xl: "p-10",
    full: "p-6 w-full h-full",
  };
  return sizeMap[props.size];
});

const radiusClasses = computed(() => {
  const radiusMap = {
    sm: "rounded",
    md: "rounded-md",
    lg: "rounded-lg",
    xl: "rounded-xl",
    "2xl": "rounded-2xl",
  };
  return radiusMap[props.radius];
});

const handleClick = () => {
  if (props.clickable) {
    emit("click");
  }
};
</script>
