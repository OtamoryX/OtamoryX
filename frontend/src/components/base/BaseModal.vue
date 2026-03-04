<template>
  <teleport to="body">
    <transition
      enter-active-class="transition-all duration-200 ease-out"
      enter-from-class="opacity-0"
      enter-to-class="opacity-100"
      leave-active-class="transition-all duration-200 ease-in"
      leave-from-class="opacity-100"
      leave-to-class="opacity-0"
    >
      <div
        v-if="show"
        class="fixed inset-0 flex items-center justify-center p-4 sm:p-6"
        :style="zIndexStyle"
        @click="handleMaskClick"
      >
        <div class="absolute inset-0 bg-black/60 backdrop-blur-sm" />

        <transition
          enter-active-class="transition-all duration-200 ease-out"
          enter-from-class="opacity-0 scale-95 translate-y-4"
          enter-to-class="opacity-100 scale-100 translate-y-0"
          leave-active-class="transition-all duration-200 ease-in"
          leave-from-class="opacity-100 scale-100 translate-y-0"
          leave-to-class="opacity-0 scale-95 translate-y-4"
        >
          <div
            v-if="show"
            :class="[
              'relative bg-[var(--bg-card)] border border-[var(--border)] rounded-xl shadow-2xl flex flex-col overflow-hidden',
              widthClass,
              maxHeightClass,
            ]"
            @click.stop
          >
            <div
              v-if="$slots.header || title"
              class="border-b border-[var(--border)] p-5 shrink-0"
            >
              <slot name="header" :title="title" :on-close="handleClose">
                <div class="flex items-center justify-between">
                  <h3 class="text-base font-semibold text-[var(--text-primary)]">
                    {{ title }}
                  </h3>
                  <button
                    v-if="closable"
                    class="text-[var(--text-tertiary)] hover:text-[var(--text-primary)] transition-colors p-1 rounded hover:bg-[var(--bg-tertiary)]"
                    @click="handleClose"
                  >
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                    </svg>
                  </button>
                </div>
              </slot>
            </div>

            <div :class="['modal-body flex-1 min-h-0', contentPadding ? 'p-5' : '']">
              <slot />
            </div>

            <div v-if="$slots.footer" class="border-t border-[var(--border)] p-5 shrink-0">
              <slot name="footer" :onClose="handleClose" />
            </div>
          </div>
        </transition>
      </div>
    </transition>
  </teleport>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted } from "vue";

interface Props {
  show: boolean;
  title?: string;
  width?: "sm" | "md" | "lg" | "xl" | "2xl" | "full";
  maxHeight?: "sm" | "md" | "lg" | "xl" | "full" | "screen";
  closable?: boolean;
  maskClosable?: boolean;
  contentPadding?: boolean;
  zIndex?: number;
}

const props = withDefaults(defineProps<Props>(), {
  width: "md",
  maxHeight: "lg",
  closable: true,
  maskClosable: true,
  contentPadding: true,
  zIndex: 50,
});

const emit = defineEmits<{ close: [] }>();

const widthClass = computed(() => {
  const widthMap = {
    sm: "max-w-sm w-full",
    md: "max-w-md w-full",
    lg: "max-w-lg w-full",
    xl: "max-w-xl w-full",
    "2xl": "max-w-2xl w-full",
    full: "max-w-[min(96vw,1440px)] w-full",
  };
  return widthMap[props.width];
});

const maxHeightClass = computed(() => {
  const heightMap = {
    sm: "max-h-60",
    md: "max-h-96",
    lg: "max-h-[40rem]",
    xl: "max-h-[48rem]",
    full: "max-h-[calc(100dvh-2rem)]",
    screen: "max-h-[100dvh]",
  };
  return heightMap[props.maxHeight];
});

const zIndexStyle = computed(() => ({ zIndex: props.zIndex }));
const handleClose = () => { emit("close"); };
const handleMaskClick = () => { if (props.maskClosable) handleClose(); };

const handleKeydown = (event: KeyboardEvent) => {
  if (event.key === "Escape" && props.show && props.closable) handleClose();
};

onMounted(() => { document.addEventListener("keydown", handleKeydown); });
onUnmounted(() => { document.removeEventListener("keydown", handleKeydown); });
</script>

<style scoped>
.modal-body {
  overflow-y: auto;
}
</style>
