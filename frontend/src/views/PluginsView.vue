<template>
  <div class="min-h-screen bg-[var(--bg-secondary)] px-4 py-6">
    <div class="max-w-6xl mx-auto">
      <!-- 页面标题 -->
      <div class="flex justify-between items-center mb-6">
        <div>
          <h1 class="text-xl font-semibold text-[var(--text-primary)]">插件管理</h1>
          <p class="text-sm text-[var(--text-secondary)] mt-0.5">管理系统插件和扩展功能</p>
        </div>
        <div class="flex gap-2">
          <button class="px-3 py-2 bg-[var(--bg-card)] border border-[var(--border)] hover:bg-[var(--bg-tertiary)] text-[var(--text-secondary)] text-sm rounded transition-colors flex items-center gap-2" @click="loadPlugins">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
            刷新
          </button>
          <button class="px-3 py-2 bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white text-sm rounded transition-colors flex items-center gap-2" @click="showInstallModal = true">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
            </svg>
            安装插件
          </button>
        </div>
      </div>

      <!-- 错误提示 -->
      <div v-if="error" class="mb-4 px-4 py-3 bg-red-500/10 border border-red-500/30 rounded text-red-500 text-sm">
        {{ error }}
        <button class="ml-2 underline text-xs" @click="loadPlugins">重试</button>
      </div>

      <!-- 加载状态 -->
      <div v-if="loading" class="bg-[var(--bg-card)] border border-[var(--border)] rounded-lg p-8 text-center">
        <div class="animate-spin rounded-full h-7 w-7 border-2 border-[var(--border)] border-t-[var(--accent)] mx-auto" />
        <p class="mt-3 text-sm text-[var(--text-secondary)]">加载中...</p>
      </div>

      <!-- 空状态 -->
      <div v-else-if="plugins.length === 0" class="bg-[var(--bg-card)] border border-[var(--border)] rounded-lg p-12 text-center">
        <svg class="w-12 h-12 mx-auto mb-3 text-[var(--text-tertiary)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
            d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4" />
        </svg>
        <p class="text-[var(--text-secondary)] mb-1">暂无插件</p>
        <p class="text-sm text-[var(--text-tertiary)]">点击"安装插件"按钮添加功能扩展</p>
      </div>

      <!-- 插件列表 -->
      <div v-else class="space-y-3">
        <div v-for="plugin in plugins" :key="plugin.id"
          class="bg-[var(--bg-card)] border border-[var(--border)] rounded-lg p-5 hover:border-[var(--text-tertiary)] transition-colors">
          <div class="flex items-start justify-between">
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2 mb-1.5">
                <h3 class="text-sm font-semibold text-[var(--text-primary)]">{{ plugin.name }}</h3>
                <span class="text-xs text-[var(--text-tertiary)]">v{{ plugin.version }}</span>
                <span :class="plugin.enabled ? 'bg-green-500/15 text-green-400' : 'bg-[var(--bg-tertiary)] text-[var(--text-tertiary)]'"
                  class="px-1.5 py-0.5 text-xs rounded">
                  {{ plugin.enabled ? "已启用" : "已禁用" }}
                </span>
              </div>
              <p v-if="plugin.description" class="text-sm text-[var(--text-secondary)] mb-2">{{ plugin.description }}</p>
              <div class="text-xs text-[var(--text-tertiary)] flex gap-4">
                <span>安装: {{ formatDate(plugin.installedAt) }}</span>
                <span>更新: {{ formatDate(plugin.updatedAt) }}</span>
              </div>
            </div>
            <div class="flex flex-col gap-1.5 ml-4 shrink-0">
              <button :disabled="toggling.has(plugin.id)"
                :class="plugin.enabled ? 'bg-red-600 hover:bg-red-700' : 'bg-green-600 hover:bg-green-700'"
                class="px-3 py-1.5 text-white text-xs rounded disabled:opacity-50 transition-colors"
                @click="togglePlugin(plugin)">
                {{ toggling.has(plugin.id) ? "操作中..." : plugin.enabled ? "禁用" : "启用" }}
              </button>
              <button class="px-3 py-1.5 bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white text-xs rounded transition-colors" @click="configurePlugin(plugin)">配置</button>
              <button :disabled="uninstalling.has(plugin.id)"
                class="px-3 py-1.5 bg-[var(--bg-tertiary)] hover:bg-[var(--border)] text-[var(--text-secondary)] text-xs rounded disabled:opacity-50 transition-colors"
                @click="confirmUninstall(plugin)">
                {{ uninstalling.has(plugin.id) ? "卸载中..." : "卸载" }}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 安装插件模态框 -->
    <div v-if="showInstallModal" class="fixed inset-0 z-50 flex items-center justify-center">
      <div class="absolute inset-0 bg-black/50" @click="cancelInstall" />
      <div class="relative bg-[var(--bg-card)] border border-[var(--border)] rounded-lg shadow-xl w-full max-w-sm mx-4 p-6">
        <h3 class="text-base font-semibold text-[var(--text-primary)] mb-5">安装插件</h3>
        <form class="space-y-4" @submit.prevent="installPluginSubmit">
          <div>
            <label class="block text-sm text-[var(--text-secondary)] mb-1.5">选择插件文件 (.tar.gz)</label>
            <input ref="fileInput" type="file" accept=".tar.gz,.tgz" required @change="onFileChange"
              class="w-full text-sm text-[var(--text-primary)] file:mr-3 file:py-1.5 file:px-3 file:rounded file:border file:border-[var(--border)] file:text-xs file:bg-[var(--bg-tertiary)] file:text-[var(--text-secondary)] hover:file:bg-[var(--border)]" />
          </div>
          <div v-if="selectedFile" class="bg-[var(--bg-tertiary)] border border-[var(--border)] p-3 rounded text-xs text-[var(--text-secondary)]">
            <p>{{ selectedFile.name }}</p>
            <p class="text-[var(--text-tertiary)]">{{ formatFileSize(selectedFile.size) }}</p>
          </div>
          <div class="flex gap-3 pt-2">
            <button type="button" class="flex-1 px-4 py-2 text-sm bg-[var(--bg-tertiary)] text-[var(--text-secondary)] rounded hover:bg-[var(--border)] transition-colors" @click="cancelInstall">取消</button>
            <button type="submit" :disabled="installing || !selectedFile" class="flex-1 px-4 py-2 text-sm bg-[var(--accent)] text-white rounded hover:bg-[var(--accent-hover)] disabled:opacity-50 transition-colors">
              {{ installing ? "安装中..." : "安装" }}
            </button>
          </div>
        </form>
      </div>
    </div>

    <!-- 配置插件模态框 -->
    <div v-if="showConfigModal" class="fixed inset-0 z-50 flex items-center justify-center">
      <div class="absolute inset-0 bg-black/50" @click="showConfigModal = false" />
      <div class="relative bg-[var(--bg-card)] border border-[var(--border)] rounded-lg shadow-xl w-full max-w-lg mx-4 p-6">
        <h3 class="text-base font-semibold text-[var(--text-primary)] mb-5">配置插件 · {{ configPlugin?.name }}</h3>
        <div>
          <label class="block text-sm text-[var(--text-secondary)] mb-1.5">配置（JSON 格式）</label>
          <textarea v-model="configJson" rows="10" placeholder="输入插件配置的 JSON 数据..."
            class="w-full px-3 py-2 bg-[var(--bg-tertiary)] border border-[var(--border)] rounded text-[var(--text-primary)] placeholder-[var(--text-tertiary)] focus:outline-none focus:border-[var(--accent)] transition-colors text-sm font-mono" />
        </div>
        <div class="flex gap-3 mt-4">
          <button class="flex-1 px-4 py-2 text-sm bg-[var(--bg-tertiary)] text-[var(--text-secondary)] rounded hover:bg-[var(--border)] transition-colors" @click="showConfigModal = false">取消</button>
          <button :disabled="configuring" class="flex-1 px-4 py-2 text-sm bg-[var(--accent)] text-white rounded hover:bg-[var(--accent-hover)] disabled:opacity-50 transition-colors" @click="savePluginConfig">
            {{ configuring ? "保存中..." : "保存配置" }}
          </button>
        </div>
      </div>
    </div>

    <!-- 卸载确认模态框 -->
    <div v-if="showUninstallModal" class="fixed inset-0 z-50 flex items-center justify-center">
      <div class="absolute inset-0 bg-black/50" @click="showUninstallModal = false" />
      <div class="relative bg-[var(--bg-card)] border border-[var(--border)] rounded-lg shadow-xl w-full max-w-sm mx-4 p-6 text-center">
        <svg class="w-10 h-10 mx-auto text-red-500 mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
            d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z" />
        </svg>
        <h3 class="text-base font-semibold text-[var(--text-primary)] mb-2">确认卸载</h3>
        <p class="text-sm text-[var(--text-secondary)] mb-5">确定要卸载插件「{{ pluginToUninstall?.name }}」吗？此操作无法撤销。</p>
        <div class="flex gap-3">
          <button class="flex-1 px-4 py-2 text-sm bg-[var(--bg-tertiary)] text-[var(--text-secondary)] rounded hover:bg-[var(--border)] transition-colors" @click="showUninstallModal = false">取消</button>
          <button :disabled="uninstalling.has(pluginToUninstall?.id || '')"
            class="flex-1 px-4 py-2 text-sm bg-red-600 text-white rounded hover:bg-red-700 disabled:opacity-50 transition-colors"
            @click="uninstallPluginConfirm">
            {{ uninstalling.has(pluginToUninstall?.id || "") ? "卸载中..." : "卸载" }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { getPlugins, installPlugin, togglePlugin as apiTogglePlugin, configurePlugin as apiConfigurePlugin, uninstallPlugin } from "@/utils/api";
import type { Plugin } from "@/types/api";

const plugins = ref<Plugin[]>([]);
const loading = ref(false);
const error = ref<string>("");
const showInstallModal = ref(false);
const showConfigModal = ref(false);
const showUninstallModal = ref(false);
const installing = ref(false);
const toggling = ref(new Set<string>());
const configuring = ref(false);
const uninstalling = ref(new Set<string>());
const fileInput = ref<HTMLInputElement>();
const selectedFile = ref<File | null>(null);
const configPlugin = ref<Plugin | null>(null);
const configJson = ref<string>("");
const pluginToUninstall = ref<Plugin | null>(null);

const loadPlugins = async () => {
  loading.value = true; error.value = "";
  try { plugins.value = await getPlugins(); }
  catch (err: any) { error.value = err.response?.data?.message || "加载插件列表失败"; }
  finally { loading.value = false; }
};

const onFileChange = (event: Event) => {
  const target = event.target as HTMLInputElement;
  const file = target.files?.[0];
  if (file) selectedFile.value = file;
};

const installPluginSubmit = async () => {
  if (!selectedFile.value) return;
  installing.value = true;
  try {
    const formData = new FormData();
    formData.append("plugin", selectedFile.value);
    await installPlugin(formData);
    showInstallModal.value = false; selectedFile.value = null; await loadPlugins();
  } catch (err: any) { error.value = err.response?.data?.message || "安装插件失败"; }
  finally { installing.value = false; }
};

const cancelInstall = () => { showInstallModal.value = false; selectedFile.value = null; if (fileInput.value) fileInput.value.value = ""; };

const togglePlugin = async (plugin: Plugin) => {
  toggling.value.add(plugin.id);
  try { await apiTogglePlugin(plugin.id); await loadPlugins(); }
  catch (err: any) { error.value = err.response?.data?.message || "切换插件状态失败"; }
  finally { toggling.value.delete(plugin.id); }
};

const configurePlugin = (plugin: Plugin) => {
  configPlugin.value = plugin;
  configJson.value = plugin.config ? JSON.stringify(plugin.config, null, 2) : "{}";
  showConfigModal.value = true;
};

const savePluginConfig = async () => {
  if (!configPlugin.value) return;
  configuring.value = true;
  try {
    let config = {};
    if (configJson.value.trim()) config = JSON.parse(configJson.value);
    await apiConfigurePlugin(configPlugin.value.id, config);
    showConfigModal.value = false; await loadPlugins();
  } catch (err: any) {
    if (err instanceof SyntaxError) error.value = "JSON 格式错误";
    else error.value = err.response?.data?.message || "保存配置失败";
  } finally { configuring.value = false; }
};

const confirmUninstall = (plugin: Plugin) => { pluginToUninstall.value = plugin; showUninstallModal.value = true; };

const uninstallPluginConfirm = async () => {
  if (!pluginToUninstall.value) return;
  uninstalling.value.add(pluginToUninstall.value.id);
  try { await uninstallPlugin(pluginToUninstall.value.id); showUninstallModal.value = false; pluginToUninstall.value = null; await loadPlugins(); }
  catch (err: any) { error.value = err.response?.data?.message || "卸载插件失败"; }
  finally { if (pluginToUninstall.value) uninstalling.value.delete(pluginToUninstall.value.id); }
};

const formatDate = (dateString: string) => new Date(dateString).toLocaleString("zh-CN");
const formatFileSize = (bytes: number) => {
  const sizes = ["B", "KB", "MB", "GB"];
  if (bytes === 0) return "0 B";
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return Math.round((bytes / Math.pow(1024, i)) * 100) / 100 + " " + sizes[i];
};

onMounted(() => { loadPlugins(); });
</script>
