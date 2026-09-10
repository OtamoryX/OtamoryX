<template>
  <GlassCard size="md" radius="lg">
    <h2 class="mb-4 text-lg font-medium text-[var(--text-primary)]">
      {{ t("appearance.title") }}
    </h2>
    <div class="space-y-6">
      <div>
        <label
          class="mb-3 block text-sm font-medium text-[var(--text-primary)]"
        >
          {{ t("appearance.themeMode") }}
        </label>
        <div class="grid grid-cols-1 gap-3 sm:grid-cols-3">
          <button
            v-for="option in themeOptions"
            :key="option.value"
            :class="[
              'rounded-lg border-2 p-4 text-left transition-all',
              theme === option.value
                ? 'border-[var(--accent)] bg-[var(--accent)]/15'
                : 'border-[var(--border)] bg-[var(--bg-tertiary)] hover:border-[var(--accent)]/30',
            ]"
            @click="emit('change-theme', option.value)"
          >
            <div class="text-sm font-medium text-[var(--text-primary)]">
              {{ option.label }}
            </div>
            <div class="mt-1 text-xs text-[var(--text-tertiary)]">
              {{ option.description }}
            </div>
          </button>
        </div>
      </div>

      <div
        class="flex items-center justify-between gap-4 rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-4"
      >
        <div>
          <div class="text-sm font-medium text-[var(--text-primary)]">
            {{ t("appearance.randomPicks.title") }}
          </div>
          <div class="mt-1 text-sm text-[var(--text-secondary)]">
            {{ t("appearance.randomPicks.description") }}
          </div>
        </div>
        <button
          :class="[
            'relative inline-flex h-6 w-11 items-center rounded-full transition-colors',
            showCarousel ? 'bg-[var(--accent)]' : 'bg-[var(--border)]',
          ]"
          @click="emit('toggle-carousel')"
        >
          <span
            :class="[
              'inline-block h-4 w-4 transform rounded-full bg-white transition-transform',
              showCarousel ? 'translate-x-6' : 'translate-x-1',
            ]"
          />
        </button>
      </div>

      <div
        class="flex items-center justify-between gap-4 rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-4"
      >
        <div>
          <div class="text-sm font-medium text-[var(--text-primary)]">
            {{ t("appearance.rowsPerPage.title") }}
          </div>
          <div class="mt-1 text-sm text-[var(--text-secondary)]">
            {{ t("appearance.rowsPerPage.description") }}
          </div>
        </div>
        <select
          :value="rowsPerPage"
          class="w-20 rounded-lg border border-[var(--border)] bg-[var(--bg-card)] px-3 py-2 text-sm text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
          @change="
            emit(
              'change-rows',
              Number(($event.target as HTMLSelectElement).value),
            )
          "
        >
          <option v-for="n in 8" :key="n + 2" :value="n + 2">
            {{ n + 2 }}
          </option>
        </select>
      </div>
    </div>
  </GlassCard>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";

import GlassCard from "@/components/base/GlassCard.vue";

interface Props {
  theme: "light" | "dark" | "system";
  showCarousel: boolean;
  rowsPerPage: number;
}

defineProps<Props>();

const { t } = useI18n();

const emit = defineEmits<{
  "change-theme": [value: "light" | "dark" | "system"];
  "toggle-carousel": [];
  "change-rows": [value: number];
}>();

const themeOptions = computed<
  Array<{
    value: "light" | "dark" | "system";
    label: string;
    description: string;
  }>
>(() => [
  {
    value: "light",
    label: t("appearance.themeOptions.light.label"),
    description: t("appearance.themeOptions.light.description"),
  },
  {
    value: "dark",
    label: t("appearance.themeOptions.dark.label"),
    description: t("appearance.themeOptions.dark.description"),
  },
  {
    value: "system",
    label: t("appearance.themeOptions.system.label"),
    description: t("appearance.themeOptions.system.description"),
  },
]);
</script>
