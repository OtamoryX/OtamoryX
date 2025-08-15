<template>
  <div class="container mx-auto px-4 py-6">
    <!-- 页面标题和操作按钮 -->
    <div class="flex justify-between items-center mb-6">
      <div>
        <h1 class="text-2xl font-bold text-gray-900">
          插件管理
        </h1>
        <p class="text-gray-600">
          管理系统插件和扩展功能
        </p>
      </div>
      <div class="flex gap-3">
        <button
          class="bg-gray-600 hover:bg-gray-700 text-white px-4 py-2 rounded-md flex items-center gap-2"
          @click="loadPlugins"
        >
          <svg
            class="w-5 h-5"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2" 
              d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
            />
          </svg>
          刷新
        </button>
        <button
          class="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-md flex items-center gap-2"
          @click="showInstallModal = true"
        >
          <svg
            class="w-5 h-5"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M12 6v6m0 0v6m0-6h6m-6 0H6"
            />
          </svg>
          安装插件
        </button>
      </div>
    </div>

    <!-- 插件列表 -->
    <div class="bg-white shadow rounded-lg overflow-hidden">
      <div
        v-if="loading"
        class="p-6 text-center"
      >
        <div
          class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600 mx-auto"
        />
        <p class="mt-2 text-gray-600">
          加载中...
        </p>
      </div>

      <div
        v-else-if="error"
        class="p-6 text-center text-red-600"
      >
        <p>{{ error }}</p>
        <button
          class="mt-2 text-blue-600 hover:text-blue-700"
          @click="loadPlugins"
        >
          重试
        </button>
      </div>

      <div
        v-else-if="plugins.length === 0"
        class="p-6 text-center text-gray-500"
      >
        <svg
          class="w-12 h-12 mx-auto mb-4 text-gray-400"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2" 
            d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"
          />
        </svg>
        <p class="text-lg font-medium mb-2">
          暂无插件
        </p>
        <p class="text-gray-600">
          点击"安装插件"按钮开始添加功能扩展
        </p>
      </div>

      <div
        v-else
        class="grid gap-6 p-6"
      >
        <div
          v-for="plugin in plugins"
          :key="plugin.id"
          class="border border-gray-200 rounded-lg p-6 hover:shadow-md transition-shadow"
        >
          <div class="flex items-start justify-between">
            <div class="flex-1">
              <div class="flex items-center gap-3 mb-2">
                <h3 class="text-lg font-semibold text-gray-900">
                  {{ plugin.name }}
                </h3>
                <span class="text-sm text-gray-500">v{{ plugin.version }}</span>
                <span
                  :class="getStatusBadgeClass(plugin.enabled)"
                  class="px-2 py-1 text-xs font-medium rounded-full"
                >
                  {{ plugin.enabled ? "已启用" : "已禁用" }}
                </span>
              </div>

              <p
                v-if="plugin.description"
                class="text-gray-600 mb-3"
              >
                {{ plugin.description }}
              </p>

              <div class="text-sm text-gray-500">
                <p>安装时间: {{ formatDate(plugin.installedAt) }}</p>
                <p>更新时间: {{ formatDate(plugin.updatedAt) }}</p>
              </div>
            </div>

            <div class="flex flex-col gap-2 ml-4">
              <button
                :disabled="toggling.has(plugin.id)"
                :class="
                  plugin.enabled
                    ? 'bg-red-600 hover:bg-red-700'
                    : 'bg-green-600 hover:bg-green-700'
                "
                class="px-3 py-1 text-white text-sm rounded-md disabled:opacity-50"
                @click="togglePlugin(plugin)"
              >
                {{
                  toggling.has(plugin.id)
                    ? "操作中..."
                    : plugin.enabled
                      ? "禁用"
                      : "启用"
                }}
              </button>

              <button
                class="px-3 py-1 bg-blue-600 hover:bg-blue-700 text-white text-sm rounded-md"
                @click="configurePlugin(plugin)"
              >
                配置
              </button>

              <button
                :disabled="uninstalling.has(plugin.id)"
                class="px-3 py-1 bg-gray-600 hover:bg-gray-700 text-white text-sm rounded-md disabled:opacity-50"
                @click="confirmUninstall(plugin)"
              >
                {{ uninstalling.has(plugin.id) ? "卸载中..." : "卸载" }}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 安装插件模态框 -->
    <div
      v-if="showInstallModal"
      class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50"
    >
      <div
        class="relative top-20 mx-auto p-5 border w-96 shadow-lg rounded-md bg-white"
      >
        <div class="mt-3">
          <h3 class="text-lg font-medium text-gray-900 mb-4 text-center">
            安装插件
          </h3>

          <form
            class="space-y-4"
            @submit.prevent="installPluginSubmit"
          >
            <div>
              <label class="block text-sm font-medium text-gray-700 mb-2">
                选择插件文件 (.tar.gz)
              </label>
              <input
                ref="fileInput"
                type="file"
                accept=".tar.gz,.tgz"
                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                required
                @change="onFileChange"
              >
            </div>

            <div
              v-if="selectedFile"
              class="bg-gray-50 p-3 rounded-md"
            >
              <p class="text-sm text-gray-600">
                已选择: {{ selectedFile.name }}
              </p>
              <p class="text-xs text-gray-500">
                大小: {{ formatFileSize(selectedFile.size) }}
              </p>
            </div>

            <div class="flex gap-3 pt-4">
              <button
                type="button"
                class="flex-1 px-4 py-2 bg-gray-300 text-gray-700 rounded-md hover:bg-gray-400"
                @click="cancelInstall"
              >
                取消
              </button>
              <button
                type="submit"
                :disabled="installing || !selectedFile"
                class="flex-1 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
              >
                {{ installing ? "安装中..." : "安装" }}
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>

    <!-- 配置插件模态框 -->
    <div
      v-if="showConfigModal"
      class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50"
    >
      <div
        class="relative top-20 mx-auto p-5 border w-1/2 max-w-2xl shadow-lg rounded-md bg-white"
      >
        <div class="mt-3">
          <h3 class="text-lg font-medium text-gray-900 mb-4 text-center">
            配置插件 - {{ configPlugin?.name }}
          </h3>

          <div class="space-y-4">
            <div>
              <label class="block text-sm font-medium text-gray-700 mb-2">
                配置 (JSON 格式)
              </label>
              <textarea
                v-model="configJson"
                rows="10"
                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-blue-500 focus:border-blue-500 font-mono text-sm"
                placeholder="输入插件配置的 JSON 数据..."
              />
            </div>

            <div class="flex gap-3 pt-4">
              <button
                class="flex-1 px-4 py-2 bg-gray-300 text-gray-700 rounded-md hover:bg-gray-400"
                @click="showConfigModal = false"
              >
                取消
              </button>
              <button
                :disabled="configuring"
                class="flex-1 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
                @click="savePluginConfig"
              >
                {{ configuring ? "保存中..." : "保存配置" }}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 卸载确认模态框 -->
    <div
      v-if="showUninstallModal"
      class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50"
    >
      <div
        class="relative top-20 mx-auto p-5 border w-96 shadow-lg rounded-md bg-white"
      >
        <div class="mt-3 text-center">
          <svg
            class="w-12 h-12 mx-auto text-red-600 mb-4"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2" 
              d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"
            />
          </svg>
          <h3 class="text-lg font-medium text-gray-900 mb-2">
            确认卸载
          </h3>
          <p class="text-gray-600 mb-4">
            确定要卸载插件 "{{ pluginToUninstall?.name }}" 吗？此操作无法撤销。
          </p>

          <div class="flex gap-3">
            <button
              class="flex-1 px-4 py-2 bg-gray-300 text-gray-700 rounded-md hover:bg-gray-400"
              @click="showUninstallModal = false"
            >
              取消
            </button>
            <button
              :disabled="uninstalling.has(pluginToUninstall?.id || '')"
              class="flex-1 px-4 py-2 bg-red-600 text-white rounded-md hover:bg-red-700 disabled:opacity-50"
              @click="uninstallPluginConfirm"
            >
              {{
                uninstalling.has(pluginToUninstall?.id || "")
                  ? "卸载中..."
                  : "卸载"
              }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import {
  getPlugins,
  installPlugin,
  togglePlugin as apiTogglePlugin,
  configurePlugin as apiConfigurePlugin,
  uninstallPlugin,
} from "@/utils/api";
import type { Plugin } from "@/types/api";

// 响应式数据
const plugins = ref<Plugin[]>([]);
const loading = ref(false);
const error = ref<string>("");

// 模态框状态
const showInstallModal = ref(false);
const showConfigModal = ref(false);
const showUninstallModal = ref(false);

// 操作状态
const installing = ref(false);
const toggling = ref(new Set<string>());
const configuring = ref(false);
const uninstalling = ref(new Set<string>());

// 文件上传
const fileInput = ref<HTMLInputElement>();
const selectedFile = ref<File | null>(null);

// 配置
const configPlugin = ref<Plugin | null>(null);
const configJson = ref<string>("");

// 卸载
const pluginToUninstall = ref<Plugin | null>(null);

// 加载插件列表
const loadPlugins = async () => {
  loading.value = true;
  error.value = "";
  try {
    plugins.value = await getPlugins();
  } catch (err: any) {
    error.value = err.response?.data?.message || "加载插件列表失败";
  } finally {
    loading.value = false;
  }
};

// 文件选择
const onFileChange = (event: Event) => {
  const target = event.target as HTMLInputElement;
  const file = target.files?.[0];
  if (file) {
    selectedFile.value = file;
  }
};

// 安装插件
const installPluginSubmit = async () => {
  if (!selectedFile.value) return;

  installing.value = true;
  try {
    const formData = new FormData();
    formData.append("plugin", selectedFile.value);

    await installPlugin(formData);
    showInstallModal.value = false;
    selectedFile.value = null;
    await loadPlugins();
  } catch (err: any) {
    error.value = err.response?.data?.message || "安装插件失败";
  } finally {
    installing.value = false;
  }
};

// 取消安装
const cancelInstall = () => {
  showInstallModal.value = false;
  selectedFile.value = null;
  if (fileInput.value) {
    fileInput.value.value = "";
  }
};

// 切换插件状态
const togglePlugin = async (plugin: Plugin) => {
  toggling.value.add(plugin.id);
  try {
    await apiTogglePlugin(plugin.id);
    await loadPlugins();
  } catch (err: any) {
    error.value = err.response?.data?.message || "切换插件状态失败";
  } finally {
    toggling.value.delete(plugin.id);
  }
};

// 配置插件
const configurePlugin = (plugin: Plugin) => {
  configPlugin.value = plugin;
  configJson.value = plugin.config
    ? JSON.stringify(plugin.config, null, 2)
    : "{}";
  showConfigModal.value = true;
};

// 保存插件配置
const savePluginConfig = async () => {
  if (!configPlugin.value) return;

  configuring.value = true;
  try {
    let config = {};
    if (configJson.value.trim()) {
      config = JSON.parse(configJson.value);
    }

    await apiConfigurePlugin(configPlugin.value.id, config);
    showConfigModal.value = false;
    await loadPlugins();
  } catch (err: any) {
    if (err instanceof SyntaxError) {
      error.value = "JSON 格式错误，请检查配置格式";
    } else {
      error.value = err.response?.data?.message || "保存配置失败";
    }
  } finally {
    configuring.value = false;
  }
};

// 确认卸载
const confirmUninstall = (plugin: Plugin) => {
  pluginToUninstall.value = plugin;
  showUninstallModal.value = true;
};

// 卸载插件
const uninstallPluginConfirm = async () => {
  if (!pluginToUninstall.value) return;

  uninstalling.value.add(pluginToUninstall.value.id);
  try {
    await uninstallPlugin(pluginToUninstall.value.id);
    showUninstallModal.value = false;
    pluginToUninstall.value = null;
    await loadPlugins();
  } catch (err: any) {
    error.value = err.response?.data?.message || "卸载插件失败";
  } finally {
    if (pluginToUninstall.value) {
      uninstalling.value.delete(pluginToUninstall.value.id);
    }
  }
};

// 工具函数
const getStatusBadgeClass = (enabled: boolean) => {
  return enabled ? "bg-green-100 text-green-800" : "bg-gray-100 text-gray-800";
};

const formatDate = (dateString: string) => {
  return new Date(dateString).toLocaleString("zh-CN");
};

const formatFileSize = (bytes: number) => {
  const sizes = ["B", "KB", "MB", "GB"];
  if (bytes === 0) return "0 B";
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return Math.round((bytes / Math.pow(1024, i)) * 100) / 100 + " " + sizes[i];
};

// 初始化
onMounted(() => {
  loadPlugins();
});
</script>
