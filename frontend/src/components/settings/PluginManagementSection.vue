<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <div>
        <h2 class="text-lg font-medium text-[var(--text-primary)]">插件管理</h2>
        <p class="mt-1 text-sm text-[var(--text-secondary)]">安装、启停和配置插件</p>
      </div>
      <GlassButton size="sm" variant="primary" @click="emit('install-plugin')">安装插件</GlassButton>
    </div>

    <GlassCard v-if="pluginsLoading" size="md" radius="lg">
      <div class="py-6 text-center text-[var(--text-secondary)]">加载中...</div>
    </GlassCard>

    <GlassCard v-else-if="!plugins || plugins.length === 0" size="md" radius="lg">
      <div class="py-6 text-center text-[var(--text-secondary)]">暂无已安装插件</div>
    </GlassCard>

    <div v-else class="grid gap-6 md:grid-cols-2 xl:grid-cols-3">
      <GlassCard v-for="plugin in plugins" :key="plugin.id" size="md" radius="lg">
        <div class="mb-4 flex items-start justify-between gap-3">
          <div>
            <div class="text-base font-semibold text-[var(--text-primary)]">{{ plugin.name }}</div>
            <div class="text-sm text-[var(--text-secondary)]">v{{ plugin.version }}</div>
          </div>
          <span
            :class="[
              'rounded-full px-2 py-1 text-xs',
              plugin.enabled ? 'bg-green-500/15 text-green-500' : 'bg-[var(--bg-tertiary)] text-[var(--text-secondary)]',
            ]"
          >
            {{ plugin.enabled ? '已启用' : '已禁用' }}
          </span>
        </div>

        <p v-if="plugin.description" class="mb-4 text-sm text-[var(--text-secondary)]">{{ plugin.description }}</p>
        <p class="mb-5 text-xs text-[var(--text-tertiary)]">安装于 {{ formatDate(plugin.installedAt) }}</p>

        <div class="flex items-center justify-between gap-2">
          <GlassButton
            :variant="plugin.enabled ? 'danger' : 'success'"
            size="sm"
            @click="emit('toggle-plugin', plugin)"
          >
            {{ plugin.enabled ? '禁用' : '启用' }}
          </GlassButton>
          <GlassButton v-if="plugin.config" variant="secondary" size="sm" @click="emit('configure-plugin', plugin)">
            配置
          </GlassButton>
        </div>
      </GlassCard>
    </div>
  </div>
</template>

<script setup lang="ts">
import GlassButton from "@/components/base/GlassButton.vue";
import GlassCard from "@/components/base/GlassCard.vue";
import type { Plugin } from "@/types/api";

interface Props {
  plugins?: Plugin[];
  pluginsLoading: boolean;
}

defineProps<Props>();

const emit = defineEmits<{
  "install-plugin": [];
  "toggle-plugin": [plugin: Plugin];
  "configure-plugin": [plugin: Plugin];
}>();

const formatDate = (dateString: string) => {
  const date = new Date(dateString);
  return date.toLocaleDateString("zh-CN", {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
};
</script>
