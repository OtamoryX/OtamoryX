<template>
  <div class="space-y-6">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <div>
        <h2 class="text-lg font-medium text-[var(--text-primary)]">插件管理</h2>
        <p class="mt-1 text-sm text-[var(--text-secondary)]">安装、启停和配置插件</p>
      </div>
      <GlassButton size="sm" variant="primary" @click="emit('install-plugin')">安装插件</GlassButton>
    </div>

    <GlassCard
      v-if="!pluginsLoading && typeOptions.length > 1"
      size="sm"
      radius="lg"
    >
      <div class="mb-2 text-xs text-[var(--text-secondary)]">类型筛选</div>
      <div class="flex flex-wrap gap-2">
        <button
          v-for="option in typeOptions"
          :key="option.value"
          type="button"
          class="rounded-full border px-3 py-1 text-xs transition-colors"
          :class="
            activeTypeFilter === option.value
              ? 'border-[var(--accent)] bg-[var(--accent)]/15 text-[var(--accent)]'
              : 'border-[var(--border)] bg-[var(--bg-tertiary)] text-[var(--text-secondary)] hover:text-[var(--text-primary)]'
          "
          @click="activeTypeFilter = option.value"
        >
          {{ option.label }}
        </button>
      </div>
    </GlassCard>

    <GlassCard v-if="pluginsLoading" size="md" radius="lg">
      <div class="py-6 text-center text-[var(--text-secondary)]">加载中...</div>
    </GlassCard>

    <GlassCard v-else-if="!plugins || plugins.length === 0" size="md" radius="lg">
      <div class="py-6 text-center text-[var(--text-secondary)]">暂无已安装插件</div>
    </GlassCard>

    <GlassCard v-else-if="filteredPlugins.length === 0" size="md" radius="lg">
      <div class="py-6 text-center text-[var(--text-secondary)]">当前筛选条件下没有插件</div>
    </GlassCard>

    <div v-else class="grid gap-6 md:grid-cols-2 xl:grid-cols-3">
      <GlassCard v-for="plugin in filteredPlugins" :key="resolvePluginId(plugin)" size="md" radius="lg">
        <div class="mb-4 flex items-start justify-between gap-3">
          <div>
            <div class="text-base font-semibold text-[var(--text-primary)]">{{ plugin.name }}</div>
            <div class="flex items-center gap-2 text-sm text-[var(--text-secondary)]">
              <span>v{{ plugin.version }}</span>
              <span class="rounded-full bg-[var(--bg-tertiary)] px-2 py-0.5 text-xs text-[var(--text-tertiary)]">
                {{ getPluginTypeLabel(plugin) }}
              </span>
            </div>
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

        <div class="mb-4 rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3 text-xs text-[var(--text-secondary)]">
          <div>
            执行次数: <span class="font-medium text-[var(--text-primary)]">{{ getExecutionCount(plugin) }}</span>
          </div>
          <div class="mt-1">
            上次执行:
            <span class="font-medium text-[var(--text-primary)]">{{ getLastExecutedAtLabel(plugin) }}</span>
          </div>
        </div>

        <div class="mb-4">
          <div class="mb-1 text-xs text-[var(--text-secondary)]">权限</div>
          <div class="flex flex-wrap gap-1.5">
            <span
              v-for="permission in getPermissionSummary(plugin)"
              :key="`${resolvePluginId(plugin)}-${permission}`"
              class="rounded-full border border-[var(--border)] bg-[var(--bg-tertiary)] px-2 py-0.5 text-xs text-[var(--text-secondary)]"
            >
              {{ permission }}
            </span>
          </div>
        </div>

        <p class="mb-5 text-xs text-[var(--text-tertiary)]">安装于 {{ getInstalledAtLabel(plugin) }}</p>

        <div class="flex flex-wrap items-center gap-2">
          <GlassButton variant="secondary" size="sm" @click="emit('view-plugin-executions', plugin)">
            执行记录
          </GlassButton>
          <GlassButton
            :variant="plugin.enabled ? 'danger' : 'success'"
            size="sm"
            @click="emit('toggle-plugin', plugin)"
          >
            {{ plugin.enabled ? '禁用' : '启用' }}
          </GlassButton>
          <GlassButton
            v-if="isPluginConfigurable(plugin)"
            variant="secondary"
            size="sm"
            @click="emit('configure-plugin', plugin)"
          >
            配置
          </GlassButton>
        </div>
      </GlassCard>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import GlassCard from "@/components/base/GlassCard.vue";
import GlassButton from "@/components/base/GlassButton.vue";
import type { Plugin } from "@/types/api";

interface PluginDetailSnapshot {
  type?: string;
  permissions?: unknown;
  executionCount?: number;
  lastExecutedAt?: string | null;
  configurable?: boolean;
}

interface Props {
  plugins?: Plugin[];
  pluginsLoading: boolean;
  pluginDetails?: Record<string, PluginDetailSnapshot>;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  "install-plugin": [];
  "toggle-plugin": [plugin: Plugin];
  "configure-plugin": [plugin: Plugin];
  "view-plugin-executions": [plugin: Plugin];
}>();

const activeTypeFilter = ref("all");

const pluginTypeLabelMap: Record<string, string> = {
  metadata: "Metadata",
  download: "Download",
  processor: "Processor",
  analyzer: "Analyzer",
  script: "Script",
  endpoint: "Endpoint",
  unknown: "Unknown",
};

const resolvePluginId = (plugin: Plugin): string => {
  const rawPlugin = plugin as Plugin & Record<string, unknown>;
  const rawId = rawPlugin.id ?? rawPlugin.plugin_id;
  return typeof rawId === "string" && rawId.length > 0 ? rawId : "";
};

const resolvePluginType = (plugin: Plugin): string => {
  const rawPlugin = plugin as Plugin & Record<string, unknown>;
  const detail = props.pluginDetails?.[resolvePluginId(plugin)];
  const rawType = rawPlugin.plugin_type ?? rawPlugin.type ?? detail?.type;
  if (typeof rawType === "string" && rawType.trim().length > 0) {
    return rawType.toLowerCase();
  }
  return "unknown";
};

const getPluginTypeLabel = (plugin: Plugin): string => {
  const type = resolvePluginType(plugin);
  return pluginTypeLabelMap[type] ?? type;
};

const isPluginConfigurable = (plugin: Plugin): boolean =>
  props.pluginDetails?.[resolvePluginId(plugin)]?.configurable !== false;

const resolveTypeLabel = (type: string): string => {
  return type === "all" ? "全部" : (pluginTypeLabelMap[type] ?? type);
};

const typeOptions = computed(() => {
  const typeSet = new Set<string>(["all"]);
  for (const plugin of props.plugins ?? []) {
    typeSet.add(resolvePluginType(plugin));
  }

  return Array.from(typeSet).map((type) => ({
    value: type,
    label: resolveTypeLabel(type),
  }));
});

const filteredPlugins = computed(() => {
  const allPlugins = props.plugins ?? [];
  if (activeTypeFilter.value === "all") {
    return allPlugins;
  }
  return allPlugins.filter((plugin) => resolvePluginType(plugin) === activeTypeFilter.value);
});

watch(typeOptions, (options) => {
  if (!options.some((option) => option.value === activeTypeFilter.value)) {
    activeTypeFilter.value = "all";
  }
});

const getExecutionCount = (plugin: Plugin): number => {
  const rawPlugin = plugin as Plugin & Record<string, unknown>;
  const detailCount = props.pluginDetails?.[resolvePluginId(plugin)]?.executionCount;
  const rawCount = rawPlugin.execution_count ?? detailCount;
  return typeof rawCount === "number" ? rawCount : 0;
};

const getLastExecutedAt = (plugin: Plugin): string | null => {
  const rawPlugin = plugin as Plugin & Record<string, unknown>;
  const detailTime = props.pluginDetails?.[resolvePluginId(plugin)]?.lastExecutedAt;
  const rawTime = rawPlugin.last_executed_at ?? detailTime;
  return typeof rawTime === "string" && rawTime.length > 0 ? rawTime : null;
};

const getLastExecutedAtLabel = (plugin: Plugin): string => {
  const raw = getLastExecutedAt(plugin);
  return raw ? formatDate(raw) : "暂无记录";
};

const getInstalledAtLabel = (plugin: Plugin): string => {
  const rawPlugin = plugin as Plugin & Record<string, unknown>;
  const installedAt = rawPlugin.installedAt ?? rawPlugin.installed_at;
  if (typeof installedAt !== "string") {
    return "-";
  }
  return formatDate(installedAt);
};

const toStringArray = (value: unknown): string[] => {
  if (!Array.isArray(value)) return [];
  return value.filter((item): item is string => typeof item === "string" && item.length > 0);
};

const getPermissionSummary = (plugin: Plugin): string[] => {
  const rawPlugin = plugin as Plugin & Record<string, unknown>;
  const rawPermissions = props.pluginDetails?.[resolvePluginId(plugin)]?.permissions ?? rawPlugin.permissions;

  if (!rawPermissions || typeof rawPermissions !== "object") {
    return ["未声明权限"];
  }

  const permissions = rawPermissions as Record<string, unknown>;
  const summary: string[] = [];

  const networkList = toStringArray(permissions.network);
  const networkEnabled = typeof permissions.network === "boolean" ? permissions.network : networkList.length > 0;
  summary.push(networkEnabled ? `网络(${networkList.length || "开放"})` : "无网络");

  const fsRead = toStringArray(permissions.filesystem_read ?? permissions.filesystemRead);
  if (fsRead.length > 0) {
    summary.push(`读文件(${fsRead.length})`);
  }

  const fsWrite = toStringArray(permissions.filesystem_write ?? permissions.filesystemWrite);
  if (fsWrite.length > 0) {
    summary.push(`写文件(${fsWrite.length})`);
  }

  const dbRead = permissions.database_read ?? permissions.databaseRead;
  if (typeof dbRead === "boolean") {
    summary.push(dbRead ? "数据库读" : "无数据库读");
  }

  const dbWrite = toStringArray(permissions.database_write ?? permissions.databaseWrite);
  if (dbWrite.length > 0) {
    summary.push(`数据库写(${dbWrite.length})`);
  }

  return summary;
};

const formatDate = (dateString: string) => {
  const date = new Date(dateString);
  if (Number.isNaN(date.getTime())) {
    return "-";
  }
  return date.toLocaleString("zh-CN", {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
};
</script>
