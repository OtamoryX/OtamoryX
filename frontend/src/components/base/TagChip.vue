<template>
  <span
    :class="[
      'inline-flex items-center px-3 py-1.5 rounded-lg text-sm transition-colors',
      colorClasses,
      removable && 'group cursor-pointer hover:bg-opacity-80',
    ]"
    @click="handleClick"
  >
    <!-- 标签内容 -->
    <span class="mr-2 truncate">{{ displayText }}</span>

    <!-- 删除按钮 -->
    <svg
      v-if="removable"
      class="w-3 h-3 opacity-0 group-hover:opacity-100 transition-opacity shrink-0"
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
  </span>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { Tag } from "@/types/api";
import { tagDisplayText } from "@/utils/tagDisplay";

interface Props {
  tag: Tag;
  removable?: boolean;
  color?: "default" | "blue" | "green" | "purple" | "red" | "yellow";
  showNamespace?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  removable: false,
  color: "default",
  showNamespace: true,
});

const emit = defineEmits<{
  remove: [tagId: string];
  click: [tag: Tag];
}>();

const displayText = computed(() => tagDisplayText(props.tag, props.showNamespace));

const colorClasses = computed(() => {
  const colorMap = {
    default:
      "bg-[var(--bg-tertiary)] border border-[var(--border)] hover:bg-[var(--bg-tertiary)] text-[var(--text-primary)]",
    blue: "bg-blue-500/20 backdrop-blur-md border border-blue-400/30 hover:bg-blue-500/30 text-blue-200",
    green:
      "bg-green-500/20 backdrop-blur-md border border-green-400/30 hover:bg-green-500/30 text-green-200",
    purple:
      "bg-purple-500/20 backdrop-blur-md border border-purple-400/30 hover:bg-purple-500/30 text-purple-200",
    red: "bg-red-500/20 backdrop-blur-md border border-red-400/30 hover:bg-red-500/30 text-red-200",
    yellow:
      "bg-yellow-500/20 backdrop-blur-md border border-yellow-400/30 hover:bg-yellow-500/30 text-yellow-200",
  };
  return colorMap[props.color];
});

const handleClick = () => {
  if (props.removable) {
    emit("remove", props.tag.id);
  } else {
    emit("click", props.tag);
  }
};
</script>

<style scoped>
.truncate {
  max-width: 200px;
}
</style>
