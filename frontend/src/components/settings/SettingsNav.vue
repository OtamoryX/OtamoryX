<template>
  <div class="space-y-4">
    <GlassCard size="sm" radius="lg" class="lg:hidden">
      <div class="mb-2 flex items-center justify-between gap-2">
        <label
          class="block text-xs font-medium uppercase tracking-wide text-[var(--text-tertiary)]"
        >
          当前分区
        </label>
        <span
          v-if="activeIsDirty"
          class="inline-flex items-center gap-1 text-xs text-amber-600 dark:text-amber-300"
        >
          <span
            class="h-1.5 w-1.5 rounded-full bg-amber-500"
            aria-hidden="true"
          />
          未保存
        </span>
      </div>
      <select
        :value="activeTab"
        :aria-label="`当前分区：${activeItem?.name ?? ''}`"
        class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-sm text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
        @change="emitSelect(($event.target as HTMLSelectElement).value)"
      >
        <option v-for="item in items" :key="item.id" :value="item.id">
          {{ item.name }}
        </option>
      </select>
    </GlassCard>

    <GlassCard size="sm" radius="lg" class="hidden lg:block">
      <div class="space-y-5">
        <div v-for="group in groupedItems" :key="group.name" class="space-y-2">
          <div
            class="px-2 text-xs font-semibold uppercase tracking-wide text-[var(--text-tertiary)]"
          >
            {{ group.name }}
          </div>
          <button
            v-for="item in group.items"
            :key="item.id"
            :class="[
              'w-full rounded-lg border px-3 py-2 text-left transition-colors',
              'relative',
              activeTab === item.id
                ? 'border-[var(--accent)] bg-[var(--accent)]/15 text-[var(--text-primary)]'
                : item.danger
                  ? 'border-[var(--border)] bg-[var(--bg-tertiary)] text-red-500 hover:border-red-400/40 hover:bg-red-500/10'
                  : 'border-[var(--border)] bg-[var(--bg-tertiary)] text-[var(--text-secondary)] hover:border-[var(--accent)]/35 hover:text-[var(--text-primary)]',
            ]"
            :aria-current="activeTab === item.id ? 'page' : undefined"
            @click="emitSelect(item.id)"
          >
            <div class="text-sm font-medium">{{ item.name }}</div>
            <div
              class="mt-1 flex items-center gap-2 text-xs text-[var(--text-tertiary)]"
            >
              <span>{{ item.description }}</span>
              <span
                v-if="isDirty(item.id)"
                class="h-1.5 w-1.5 shrink-0 rounded-full bg-amber-500"
                title="有未保存的更改"
                aria-label="有未保存的更改"
              />
            </div>
          </button>
        </div>
      </div>
    </GlassCard>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import GlassCard from "@/components/base/GlassCard.vue";

export interface SettingsNavItem {
  id: string;
  name: string;
  description: string;
  group?: string;
  danger?: boolean;
}

interface Props {
  items: SettingsNavItem[];
  activeTab: string;
  dirtyTabs?: readonly string[];
}

const props = defineProps<Props>();

const emit = defineEmits<{
  select: [tabId: string];
}>();

const groupedItems = computed(() => {
  const groupMap = new Map<string, SettingsNavItem[]>();
  for (const item of props.items) {
    const groupName = item.group || "常规";
    const bucket = groupMap.get(groupName);
    if (bucket) {
      bucket.push(item);
    } else {
      groupMap.set(groupName, [item]);
    }
  }

  return Array.from(groupMap.entries()).map(([name, items]) => ({
    name,
    items,
  }));
});

const isDirty = (tabId: string) => props.dirtyTabs?.includes(tabId) ?? false;
const activeIsDirty = computed(() => isDirty(props.activeTab));
const activeItem = computed(() =>
  props.items.find((item) => item.id === props.activeTab),
);

const emitSelect = (tabId: string) => {
  emit("select", tabId);
};
</script>
