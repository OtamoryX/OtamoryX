<template>
  <div class="space-y-4">
    <GlassCard size="sm" radius="lg" class="lg:hidden">
      <label class="mb-2 block text-xs font-medium uppercase tracking-wide text-[var(--text-tertiary)]">
        当前分区
      </label>
      <select
        :value="activeTab"
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
          <div class="px-2 text-xs font-semibold uppercase tracking-wide text-[var(--text-tertiary)]">
            {{ group.name }}
          </div>
          <button
            v-for="item in group.items"
            :key="item.id"
            :class="[
              'w-full rounded-lg border px-3 py-2 text-left transition-colors',
              activeTab === item.id
                ? 'border-[var(--accent)] bg-[var(--accent)]/15 text-[var(--text-primary)]'
                : item.danger
                  ? 'border-[var(--border)] bg-[var(--bg-tertiary)] text-red-500 hover:border-red-400/40 hover:bg-red-500/10'
                  : 'border-[var(--border)] bg-[var(--bg-tertiary)] text-[var(--text-secondary)] hover:border-[var(--accent)]/35 hover:text-[var(--text-primary)]',
            ]"
            @click="emitSelect(item.id)"
          >
            <div class="text-sm font-medium">{{ item.name }}</div>
            <div class="mt-1 text-xs text-[var(--text-tertiary)]">{{ item.description }}</div>
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

  return Array.from(groupMap.entries()).map(([name, items]) => ({ name, items }));
});

const emitSelect = (tabId: string) => {
  emit("select", tabId);
};
</script>
