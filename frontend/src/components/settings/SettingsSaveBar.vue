<template>
  <div
    class="sticky top-3 z-20 flex flex-wrap items-center justify-between gap-3 border px-4 py-3 shadow-sm backdrop-blur"
    :class="statusClasses"
    role="status"
    aria-live="polite"
  >
    <div class="flex min-w-0 items-start gap-2.5">
      <ExclamationCircleIcon
        v-if="error"
        class="mt-0.5 h-5 w-5 shrink-0 text-red-500"
        aria-hidden="true"
      />
      <PencilIcon
        v-else-if="dirty"
        class="mt-0.5 h-5 w-5 shrink-0 text-amber-500"
        aria-hidden="true"
      />
      <CheckCircleIcon
        v-else
        class="mt-0.5 h-5 w-5 shrink-0 text-green-600 dark:text-green-400"
        aria-hidden="true"
      />
      <div class="min-w-0">
        <div class="text-sm font-medium text-[var(--text-primary)]">
          {{
            error
              ? errorTitle || "保存失败"
              : dirty
                ? "有未保存的更改"
                : "所有更改已保存"
          }}
        </div>
        <p class="mt-0.5 text-xs text-[var(--text-secondary)]">
          {{
            error ||
            (dirty
              ? "保存后才会应用到服务器。"
              : savedMessage || "修改配置后，请使用此处的保存按钮应用更改。")
          }}
        </p>
      </div>
    </div>

    <div class="flex shrink-0 items-center gap-2">
      <GlassButton
        variant="ghost"
        size="sm"
        class-name="min-w-[5.5rem]"
        :disabled="saving || !dirty"
        @click="emit('discard')"
      >
        放弃更改
      </GlassButton>
      <GlassButton
        variant="primary"
        size="sm"
        class-name="min-w-[6.5rem]"
        :disabled="saving || !dirty"
        :loading="saving"
        loading-text="保存中..."
        @click="emit('save')"
      >
        保存更改
      </GlassButton>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import {
  CheckCircleIcon,
  ExclamationCircleIcon,
  PencilIcon,
} from "@heroicons/vue/24/outline";
import GlassButton from "@/components/base/GlassButton.vue";

const props = defineProps<{
  dirty: boolean;
  saving: boolean;
  savedMessage?: string | null;
  error?: string | null;
  errorTitle?: string;
}>();

const emit = defineEmits<{
  save: [];
  discard: [];
}>();

const statusClasses = computed(() => {
  if (props.error) {
    return "border-red-400/40 bg-red-500/5";
  }
  if (props.dirty) {
    return "border-amber-400/40 bg-amber-500/5";
  }
  return "border-green-400/30 bg-[var(--bg-card)]/95";
});
</script>
