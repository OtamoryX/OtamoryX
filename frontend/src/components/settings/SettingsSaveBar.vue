<template>
  <div
    class="sticky top-3 z-20 flex flex-wrap items-center justify-between gap-3 border border-[var(--border)] bg-[var(--bg-card)]/95 px-4 py-3 shadow-sm backdrop-blur"
  >
    <div class="min-w-0">
      <div class="text-sm font-medium text-[var(--text-primary)]">
        {{ dirty ? "有未保存的更改" : "所有更改已保存" }}
      </div>
      <p class="mt-0.5 text-xs text-[var(--text-secondary)]">
        {{
          dirty
            ? "保存后才会应用到服务器。"
            : savedMessage || "修改配置后，请使用此处的保存按钮应用更改。"
        }}
      </p>
    </div>

    <div class="flex shrink-0 items-center gap-2">
      <GlassButton
        variant="ghost"
        size="sm"
        :disabled="saving || !dirty"
        @click="emit('discard')"
      >
        放弃更改
      </GlassButton>
      <GlassButton
        variant="primary"
        size="sm"
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
import GlassButton from "@/components/base/GlassButton.vue";

defineProps<{
  dirty: boolean;
  saving: boolean;
  savedMessage?: string | null;
}>();

const emit = defineEmits<{
  save: [];
  discard: [];
}>();
</script>
