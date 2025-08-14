<template>
  <div
    :class="[
      'glass-card relative overflow-hidden transition-all duration-300',
      'bg-white/10 backdrop-blur-md border border-white/20',
      'shadow-lg hover:shadow-2xl hover:bg-white/15',
      sizeClasses,
      radiusClasses,
      clickable && 'cursor-pointer hover:scale-[1.02] active:scale-[0.98]',
      glowEffect && 'glass-glow',
      className,
    ]"
    @click="handleClick"
  >
    <!-- 装饰性光泽效果 -->
    <div v-if="shineEffect"
class="glass-shine absolute inset-0 opacity-20" />

    <!-- 内容插槽 -->
    <slot />

    <!-- 底部模糊边框 -->
    <div
      v-if="bottomBorder"
      class="absolute bottom-0 left-0 right-0 h-px bg-gradient-to-r from-transparent via-white/30 to-transparent"
    />
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
    sm: "rounded-md",
    md: "rounded-lg",
    lg: "rounded-xl",
    xl: "rounded-2xl",
    "2xl": "rounded-3xl",
  };
  return radiusMap[props.radius];
});

const handleClick = () => {
  if (props.clickable) {
    emit("click");
  }
};
</script>

<style scoped>
.glass-card {
  /* 提升毛玻璃效果 */
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
}

.glass-card::before {
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

.glass-glow {
  box-shadow:
    0 8px 32px rgba(31, 38, 135, 0.37),
    inset 0 1px 0 rgba(255, 255, 255, 0.1),
    inset 0 -1px 0 rgba(0, 0, 0, 0.1);
}

.glass-glow:hover {
  box-shadow:
    0 12px 48px rgba(31, 38, 135, 0.5),
    inset 0 1px 0 rgba(255, 255, 255, 0.15),
    inset 0 -1px 0 rgba(0, 0, 0, 0.15);
}

.glass-shine {
  background: linear-gradient(
    45deg,
    transparent 30%,
    rgba(255, 255, 255, 0.1) 50%,
    transparent 70%
  );
  transform: translateX(-100%) rotate(45deg);
  transition: transform 0.6s ease-in-out;
}

.glass-card:hover .glass-shine {
  transform: translateX(100%) rotate(45deg);
}

/* 响应式调整 */
@media (max-width: 640px) {
  .glass-card {
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
  }
}
</style>
