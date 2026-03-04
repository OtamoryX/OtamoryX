<template>
  <BasePageView theme="settings">
    <div class="settings-view w-full p-4 sm:p-6">
      <div class="mx-auto w-full max-w-7xl">
        <GlassCard size="sm" radius="lg">
          <div class="flex items-start justify-between gap-4">
            <div>
              <h1 class="text-2xl font-bold text-[var(--text-primary)]">{{ pageTitle }}</h1>
              <p class="mt-1 text-sm text-[var(--text-secondary)]">{{ pageSubtitle }}</p>
            </div>
          </div>
        </GlassCard>

        <div class="mt-6 grid grid-cols-1 gap-6 lg:grid-cols-[260px_minmax(0,1fr)]">
          <SettingsNav :items="tabs" :active-tab="activeTab" @select="setActiveTab" />

          <div class="min-w-0 pb-20">
            <AppearanceSettingsSection
              v-if="isUserSettingsRoute && activeTab === 'appearance'"
              :theme="theme"
              :show-carousel="libraryStore.showCarousel"
              :rows-per-page="libraryStore.rowsPerPage"
              @change-theme="setTheme"
              @toggle-carousel="libraryStore.setShowCarousel(!libraryStore.showCarousel)"
              @change-rows="libraryStore.setRowsPerPage"
            />

            <SystemSettingsSection
              v-if="isAdminSettingsRoute && activeTab === 'system'"
              :system-settings="systemSettings"
              :cache-settings="cacheSettings"
              :scan-settings="scanSettings"
              :cache-status="cacheStatus"
              :cache-strategy-description="getCacheStrategyDescription()"
              :system-loading="systemLoading"
              :scan-loading="scanLoading"
              :scan-result="scanResult"
              :is-clearing-cache="isClearingCache"
              :clearing-cache-scope="clearingCacheScope"
              @select-comics-path="selectComicsPath"
              @select-cache-path="selectCachePath"
              @save-system="saveSystemSettings"
              @save-scan="saveScanSettings"
              @manual-scan="handleManualScan"
              @refresh-cache-status="loadCacheStatus"
              @clear-cache="clearCache"
              @cache-strategy-change="handleCacheStrategyChange"
            />

            <UserManagementSection
              v-if="isAdminSettingsRoute && activeTab === 'users'"
              :users="users"
              :users-loading="usersLoading"
              @create-user="showCreateUserModal = true"
              @edit-user="editUser"
              @delete-user="confirmDeleteUser"
            />

            <PluginManagementSection
              v-if="isAdminSettingsRoute && activeTab === 'plugins'"
              :plugins="plugins"
              :plugins-loading="pluginsLoading"
              @install-plugin="showInstallPluginModal = true"
              @toggle-plugin="handleTogglePlugin"
              @configure-plugin="configurePlugin"
            />

            <BatchOperationsSection
              v-if="isAdminSettingsRoute && activeTab === 'batch'"
              :categories="categories"
              :tags="tags"
              :batch-delete-form="batchDeleteForm"
              :batch-operation-loading="batchOperationLoading"
              :batch-operation-history="batchOperationHistory"
              @delete-archives="handleBatchDeleteArchives"
              @delete-category="handleBatchDeleteCategoryArchives"
              @delete-tag="handleBatchDeleteTagArchives"
              @prune-tags="handlePruneTags"
              @prune-categories="handlePruneCategories"
              @clear-history="batchOperationHistory = []"
            />

            <AISettingsSection
              v-if="isAdminSettingsRoute && activeTab === 'ai'"
              :ai-settings="aiSettings"
              :ai-status="aiStatus"
              :ai-loading="aiLoading"
              @save="saveAISettings"
            />
          </div>
        </div>
      </div>

      <BaseModal
        :show="showCreateUserModal"
        title="创建用户"
        width="md"
        :z-index="9999"
        @close="showCreateUserModal = false"
      >
        <form class="space-y-4" @submit.prevent="handleCreateUser">
          <GlassInput v-model="createUserForm.username" label="用户名" type="text" required placeholder="输入用户名" />
          <GlassInput v-model="createUserForm.email" label="邮箱" type="email" placeholder="输入邮箱（可选）" />
          <GlassInput
            v-model="createUserForm.password"
            label="密码"
            type="password"
            required
            placeholder="输入密码"
          />

          <div class="flex justify-end gap-2">
            <GlassButton variant="ghost" @click="showCreateUserModal = false">取消</GlassButton>
            <GlassButton type="submit" :loading="createUserLoading" loading-text="创建中..." variant="primary">创建</GlassButton>
          </div>
        </form>
      </BaseModal>

      <BaseModal
        :show="showInstallPluginModal"
        title="安装插件"
        width="md"
        :z-index="9999"
        @close="showInstallPluginModal = false"
      >
        <form class="space-y-4" @submit.prevent="handleInstallPlugin">
          <div>
            <label class="mb-2 block text-sm font-medium text-[var(--text-primary)]">插件文件 (.zip)</label>
            <input
              ref="pluginFileInput"
              type="file"
              accept=".zip"
              required
              class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-sm text-[var(--text-primary)] focus:border-[var(--accent)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]/30"
            />
          </div>

          <div class="flex justify-end gap-2">
            <GlassButton variant="ghost" @click="showInstallPluginModal = false">取消</GlassButton>
            <GlassButton type="submit" :loading="installPluginLoading" loading-text="安装中..." variant="primary">安装</GlassButton>
          </div>
        </form>
      </BaseModal>

      <ConfirmModal
        :show="confirmDialog.show"
        :title="confirmDialog.title"
        :message="confirmDialog.message"
        :type="confirmDialog.type"
        :confirm-text="confirmDialog.confirmText"
        :cancel-text="confirmDialog.cancelText"
        :show-cancel="confirmDialog.showCancel"
        @close="handleConfirmDialogClose"
        @confirm="handleConfirmDialogConfirm"
      />

      <DirectoryBrowser
        :is-open="showDirectoryBrowser"
        :initial-path="directoryBrowserType === 'comics' ? systemSettings.comicsPath : cacheSettings.cachePath"
        @close="closeDirectoryBrowser"
        @select="handleDirectorySelected"
      />
    </div>
  </BasePageView>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useQuery, useQueryClient } from "@tanstack/vue-query";
import { useRoute, useRouter } from "vue-router";
import BasePageView from "@/components/layout/BasePageView.vue";
import BaseModal from "@/components/base/BaseModal.vue";
import ConfirmModal from "@/components/common/ConfirmModal.vue";
import DirectoryBrowser from "@/components/DirectoryBrowser.vue";
import GlassButton from "@/components/base/GlassButton.vue";
import GlassCard from "@/components/base/GlassCard.vue";
import GlassInput from "@/components/base/GlassInput.vue";
import SettingsNav, { type SettingsNavItem } from "@/components/settings/SettingsNav.vue";
import AppearanceSettingsSection from "@/components/settings/AppearanceSettingsSection.vue";
import SystemSettingsSection from "@/components/settings/SystemSettingsSection.vue";
import UserManagementSection from "@/components/settings/UserManagementSection.vue";
import PluginManagementSection from "@/components/settings/PluginManagementSection.vue";
import BatchOperationsSection from "@/components/settings/BatchOperationsSection.vue";
import AISettingsSection from "@/components/settings/AISettingsSection.vue";
import { useTheme } from "@/composables/useTheme";
import { useLibraryStore } from "@/stores/library";
import {
  batchDeleteArchives,
  batchDeleteCategoryArchives,
  batchDeleteTagArchives,
  clearCache as apiClearCache,
  configureCache,
  createUser,
  deleteUser,
  getAISettings,
  getAIStatus,
  getCacheStatus,
  getCategories,
  getPlugins,
  getScanSettings,
  getSettings,
  getTags,
  getUsers,
  installPlugin,
  pruneCategories,
  pruneTags,
  togglePlugin,
  triggerScan,
  updateAISettings,
  updateScanSettings,
  updateSettings,
} from "@/utils/api";
import type { CacheClearScope } from "@/utils/api";
import type { AISettings, Plugin, ScanSettings, SystemSettings, User } from "@/types/api";

interface CacheSettingsForm {
  cachePath: string;
  maxSize: number;
  quality: number;
  format: string;
  strategy: string;
  customConfig: {
    maxMemoryMb: number;
    maxCachedArchives: number;
    cacheTtlHours: number;
    preloadPrevPages: number;
    preloadNextPages: number;
  };
}

interface CacheStatus {
  current_strategy: string;
  stats: {
    hit_rate: number;
    memory_usage_mb: number;
    cached_archives: number;
  };
}

interface BatchOperationRecord {
  operation: string;
  timestamp: string;
  success: boolean;
  result: string;
}

const queryClient = useQueryClient();
const route = useRoute();
const router = useRouter();
const { theme, setTheme } = useTheme();
const libraryStore = useLibraryStore();

const ADMIN_TABS: SettingsNavItem[] = [
  {
    id: "system",
    name: "系统配置",
    description: "漫画库、缓存、扫描策略",
    group: "基础配置",
  },
  {
    id: "users",
    name: "用户管理",
    description: "创建与删除用户账号",
    group: "账号与扩展",
  },
  {
    id: "plugins",
    name: "插件管理",
    description: "安装、启用、配置插件",
    group: "账号与扩展",
  },
  {
    id: "ai",
    name: "AI 自动标签",
    description: "智能任务调度与阈值",
    group: "智能处理",
  },
  {
    id: "batch",
    name: "批量维护",
    description: "清理与批量删除等高风险操作",
    group: "维护与风险",
    danger: true,
  },
];

const USER_TABS: SettingsNavItem[] = [
  {
    id: "appearance",
    name: "外观",
    description: "主题与书库展示偏好",
    group: "个人偏好",
  },
];

const activeTab = ref("appearance");
const isAdminSettingsRoute = computed(() => route.name === "admin-settings");
const isUserSettingsRoute = computed(() => route.name === "settings");

const pageTitle = computed(() => (isAdminSettingsRoute.value ? "管理设置" : "个人设置"));
const pageSubtitle = computed(() =>
  isAdminSettingsRoute.value
    ? "按分区管理系统配置，危险操作集中在批量维护。"
    : "即时调整你的主题和书库显示偏好。",
);

const tabs = computed(() => (isAdminSettingsRoute.value ? ADMIN_TABS : USER_TABS));

const resolveTabFromQuery = (queryTab: unknown, isAdmin: boolean): string => {
  const rawTab = Array.isArray(queryTab) ? queryTab[0] : queryTab;
  const candidate = typeof rawTab === "string" ? rawTab : "";
  const allowed = (isAdmin ? ADMIN_TABS : USER_TABS).map((item) => item.id);
  const fallback = isAdmin ? "system" : "appearance";
  return allowed.includes(candidate) ? candidate : fallback;
};

const setActiveTab = (tabId: string) => {
  activeTab.value = resolveTabFromQuery(tabId, isAdminSettingsRoute.value);
};

watch(
  () => [route.name, route.query.tab],
  ([routeName, queryTab]) => {
    activeTab.value = resolveTabFromQuery(queryTab, routeName === "admin-settings");
  },
  { immediate: true },
);

watch(
  () => activeTab.value,
  (tab) => {
    const currentTabQuery = Array.isArray(route.query.tab) ? route.query.tab[0] : route.query.tab;

    if (isAdminSettingsRoute.value) {
      if (currentTabQuery !== tab) {
        void router.replace({ name: "admin-settings", query: { ...route.query, tab } });
      }
      return;
    }

    if (currentTabQuery !== undefined) {
      const { tab: _unusedTab, ...restQuery } = route.query;
      void router.replace({ name: "settings", query: restQuery });
    }
  },
);

const systemSettings = ref<SystemSettings>({
  comicsPath: "./comics",
  supportedFormats: ["cbz", "cbr", "cb7", "zip", "rar"],
  maxFileSize: 100,
  imageCacheSize: 1024,
  imageCachePath: "./data/cache",
  scanOnStartup: true,
  scanSettings: {
    enabled: true,
    recursive: true,
    ignoreHidden: true,
    realtimeMonitoring: false,
  },
});

const cacheSettings = ref<CacheSettingsForm>({
  cachePath: "./data/cache",
  maxSize: 1,
  quality: 85,
  format: "WebP",
  strategy: "balanced",
  customConfig: {
    maxMemoryMb: 512,
    maxCachedArchives: 30,
    cacheTtlHours: 24,
    preloadPrevPages: 2,
    preloadNextPages: 3,
  },
});

const scanSettings = ref<ScanSettings>({
  enabled: true,
  recursive: true,
  ignoreHidden: true,
  realtimeMonitoring: false,
});

const aiSettings = ref<AISettings>({
  enabled: false,
  autoApplyThreshold: 0.8,
  processingSchedule: "batch",
  maxConcurrentTasks: 2,
  enabledAnalyzers: [],
});

const systemLoading = ref(false);
const scanLoading = ref(false);
const scanResult = ref<{ success: boolean; message: string } | null>(null);
const aiLoading = ref(false);
const cacheStatus = ref<CacheStatus | null>(null);
const clearingCacheScope = ref<CacheClearScope | null>(null);
const isClearingCache = computed(() => clearingCacheScope.value !== null);

const showCreateUserModal = ref(false);
const createUserForm = ref({ username: "", email: "", password: "" });
const createUserLoading = ref(false);

const showInstallPluginModal = ref(false);
const pluginFileInput = ref<HTMLInputElement>();
const installPluginLoading = ref(false);

const batchOperationLoading = ref(false);
const batchDeleteForm = ref({ archiveIds: "", categoryId: "", tagId: "" });
const batchOperationHistory = ref<BatchOperationRecord[]>([]);

const getErrorMessage = (error: unknown, fallback: string): string => {
  if (typeof error !== "object" || error === null) return fallback;
  const withResponse = error as { response?: { data?: { message?: string } } };
  const responseMessage = withResponse.response?.data?.message;
  if (typeof responseMessage === "string" && responseMessage) return responseMessage;

  const withMessage = error as { message?: string };
  if (typeof withMessage.message === "string" && withMessage.message) return withMessage.message;

  return fallback;
};

type ConfirmDialogType = "default" | "danger" | "warning" | "info";

interface ConfirmDialogOptions {
  title?: string;
  message: string;
  type?: ConfirmDialogType;
  confirmText?: string;
  cancelText?: string;
  showCancel?: boolean;
}

const confirmDialog = ref({
  show: false,
  title: "确认操作",
  message: "",
  type: "default" as ConfirmDialogType,
  confirmText: "确认",
  cancelText: "取消",
  showCancel: true,
});

let confirmDialogResolver: ((result: boolean) => void) | null = null;

const askForConfirmation = (options: ConfirmDialogOptions): Promise<boolean> => {
  if (confirmDialogResolver) {
    confirmDialogResolver(false);
    confirmDialogResolver = null;
  }

  confirmDialog.value = {
    show: true,
    title: options.title ?? "确认操作",
    message: options.message,
    type: options.type ?? "default",
    confirmText: options.confirmText ?? "确认",
    cancelText: options.cancelText ?? "取消",
    showCancel: options.showCancel ?? true,
  };

  return new Promise((resolve) => {
    confirmDialogResolver = resolve;
  });
};

const resolveConfirmDialog = (result: boolean) => {
  confirmDialog.value.show = false;
  if (confirmDialogResolver) {
    confirmDialogResolver(result);
    confirmDialogResolver = null;
  }
};

const handleConfirmDialogClose = () => {
  resolveConfirmDialog(false);
};

const handleConfirmDialogConfirm = () => {
  resolveConfirmDialog(true);
};

const showInfoDialog = async (title: string, message: string) => {
  await askForConfirmation({
    title,
    message,
    type: "info",
    confirmText: "知道了",
    showCancel: false,
  });
};

const usersQuery = useQuery({
  queryKey: ["users"],
  queryFn: getUsers,
  enabled: computed(() => isAdminSettingsRoute.value && activeTab.value === "users"),
});

const pluginsQuery = useQuery({
  queryKey: ["plugins"],
  queryFn: getPlugins,
  enabled: computed(() => isAdminSettingsRoute.value && activeTab.value === "plugins"),
});

const aiStatusQuery = useQuery({
  queryKey: ["ai-status"],
  queryFn: getAIStatus,
  enabled: computed(() => isAdminSettingsRoute.value && activeTab.value === "ai"),
  refetchInterval: 5000,
});

const categoriesQuery = useQuery({
  queryKey: ["categories"],
  queryFn: getCategories,
  enabled: computed(() => isAdminSettingsRoute.value && activeTab.value === "batch"),
});

const tagsQuery = useQuery({
  queryKey: ["tags"],
  queryFn: getTags,
  enabled: computed(() => isAdminSettingsRoute.value && activeTab.value === "batch"),
});

const users = computed(() => usersQuery.data.value ?? []);
const usersLoading = computed(() => usersQuery.isLoading.value);
const plugins = computed(() => pluginsQuery.data.value ?? []);
const pluginsLoading = computed(() => pluginsQuery.isLoading.value);
const aiStatus = computed(() => aiStatusQuery.data.value);
const categories = computed(() => categoriesQuery.data.value ?? []);
const tags = computed(() => tagsQuery.data.value ?? []);

const showDirectoryBrowser = ref(false);
const directoryBrowserType = ref<"comics" | "cache">("comics");

const selectComicsPath = () => {
  directoryBrowserType.value = "comics";
  showDirectoryBrowser.value = true;
};

const selectCachePath = () => {
  directoryBrowserType.value = "cache";
  showDirectoryBrowser.value = true;
};

const handleDirectorySelected = (path: string) => {
  if (directoryBrowserType.value === "comics") {
    systemSettings.value.comicsPath = path;
  } else {
    cacheSettings.value.cachePath = path;
  }
  showDirectoryBrowser.value = false;
};

const closeDirectoryBrowser = () => {
  showDirectoryBrowser.value = false;
};

const getCacheStrategyDescription = () => {
  switch (cacheSettings.value.strategy) {
    case "conservative":
      return "保守策略：低内存、短缓存，适合资源受限环境";
    case "balanced":
      return "平衡策略：通用默认策略，适配大多数场景";
    case "aggressive":
      return "激进策略：高内存、长缓存，适合高性能环境";
    case "custom":
      return "自定义策略：按需精细调整缓存参数";
    default:
      return "";
  }
};

const handleCacheStrategyChange = () => {
  switch (cacheSettings.value.strategy) {
    case "conservative":
      cacheSettings.value.customConfig = {
        maxMemoryMb: 256,
        maxCachedArchives: 10,
        cacheTtlHours: 6,
        preloadPrevPages: 1,
        preloadNextPages: 2,
      };
      break;
    case "balanced":
      cacheSettings.value.customConfig = {
        maxMemoryMb: 512,
        maxCachedArchives: 30,
        cacheTtlHours: 24,
        preloadPrevPages: 2,
        preloadNextPages: 3,
      };
      break;
    case "aggressive":
      cacheSettings.value.customConfig = {
        maxMemoryMb: 1024,
        maxCachedArchives: 50,
        cacheTtlHours: 168,
        preloadPrevPages: 3,
        preloadNextPages: 5,
      };
      break;
  }
};

const saveSystemSettings = async () => {
  systemLoading.value = true;
  try {
    await updateSettings({
      ...systemSettings.value,
      maxFileSize: systemSettings.value.maxFileSize * 1024 * 1024,
      imageCacheSize: cacheSettings.value.maxSize * 1024 * 1024 * 1024,
      imageCachePath: cacheSettings.value.cachePath,
      scanSettings: scanSettings.value,
      imageCacheQuality: cacheSettings.value.quality,
      imageCacheFormat: cacheSettings.value.format,
    });

    await configureCache({
      strategy: cacheSettings.value.strategy === "custom" ? undefined : cacheSettings.value.strategy,
      custom_config:
        cacheSettings.value.strategy === "custom"
          ? {
              max_memory_mb: cacheSettings.value.customConfig.maxMemoryMb,
              max_cached_archives: cacheSettings.value.customConfig.maxCachedArchives,
              cache_ttl_hours: cacheSettings.value.customConfig.cacheTtlHours,
              preload_prev_pages: cacheSettings.value.customConfig.preloadPrevPages,
              preload_next_pages: cacheSettings.value.customConfig.preloadNextPages,
            }
          : undefined,
    });

    await showInfoDialog("操作成功", "系统与缓存配置已保存");
  } catch (error) {
    console.error("保存设置失败:", error);
    await showInfoDialog("操作失败", "保存失败，请稍后重试");
  } finally {
    systemLoading.value = false;
  }
};

const loadCacheStatus = async () => {
  try {
    cacheStatus.value = await getCacheStatus();
  } catch (error) {
    console.error("加载缓存状态失败:", error);
  }
};

const clearCache = async (scope: CacheClearScope) => {
  const scopeNameMap: Record<CacheClearScope, string> = {
    pages: "阅读缓存",
    covers: "封面缓存",
    all: "全部缓存",
  };

  const confirmMessageMap: Record<CacheClearScope, string> = {
    pages: "确定要清理阅读缓存吗？这不会影响列表封面。",
    covers: "确定要清理封面缓存吗？清理后封面会重新生成。",
    all: "确定要清空全部缓存吗？包括阅读缓存和封面缓存。",
  };

  const confirmed = await askForConfirmation({
    title: "确认清理缓存",
    message: confirmMessageMap[scope],
    type: scope === "all" ? "danger" : "warning",
    confirmText: "确认清理",
  });

  if (!confirmed) return;

  clearingCacheScope.value = scope;
  try {
    const result = await apiClearCache(scope);
    await loadCacheStatus();

    if (result.success) {
      await showInfoDialog("操作成功", `${scopeNameMap[scope]}已成功清理`);
    } else {
      await showInfoDialog("操作完成", result.message || `${scopeNameMap[scope]}已清理`);
    }
  } catch (error) {
    console.error("清理缓存失败:", error);
    await showInfoDialog("操作失败", `清理${scopeNameMap[scope]}失败`);
  } finally {
    clearingCacheScope.value = null;
  }
};

const saveAISettings = async () => {
  aiLoading.value = true;
  try {
    await updateAISettings(aiSettings.value);
    await showInfoDialog("操作成功", "AI 设置已保存");
  } catch (error) {
    console.error("保存AI设置失败:", error);
    await showInfoDialog("操作失败", "保存失败，请稍后重试");
  } finally {
    aiLoading.value = false;
  }
};

const handleCreateUser = async () => {
  const username = createUserForm.value.username.trim();
  const email = createUserForm.value.email.trim();
  if (!username || !createUserForm.value.password) return;

  createUserLoading.value = true;
  try {
    await createUser({
      username,
      password: createUserForm.value.password,
      email: email || undefined,
    });

    await queryClient.invalidateQueries({ queryKey: ["users"] });
    showCreateUserModal.value = false;
    createUserForm.value = { username: "", email: "", password: "" };
    await showInfoDialog("操作成功", "用户创建成功");
  } catch (error: unknown) {
    console.error("创建用户失败:", error);
    await showInfoDialog("操作失败", getErrorMessage(error, "创建用户失败"));
  } finally {
    createUserLoading.value = false;
  }
};

const editUser = async (_user: User) => {
  await showInfoDialog("提示", "编辑用户功能将在后续版本补齐");
};

const confirmDeleteUser = async (user: User) => {
  const confirmed = await askForConfirmation({
    title: "确认删除用户",
    message: `确定要删除用户 \"${user.username}\" 吗？`,
    type: "danger",
    confirmText: "删除",
  });

  if (!confirmed) return;

  try {
    await deleteUser(user.id);
    await queryClient.invalidateQueries({ queryKey: ["users"] });
  } catch (error) {
    console.error("删除用户失败:", error);
    await showInfoDialog("操作失败", getErrorMessage(error, "删除用户失败"));
  }
};

const handleInstallPlugin = async () => {
  if (!pluginFileInput.value?.files?.[0]) return;

  installPluginLoading.value = true;
  try {
    const formData = new FormData();
    formData.append("plugin", pluginFileInput.value.files[0]);

    await installPlugin(formData);
    await queryClient.invalidateQueries({ queryKey: ["plugins"] });

    showInstallPluginModal.value = false;
    if (pluginFileInput.value) {
      pluginFileInput.value.value = "";
    }
    await showInfoDialog("操作成功", "插件安装成功");
  } catch (error) {
    console.error("安装插件失败:", error);
    await showInfoDialog("操作失败", "安装插件失败");
  } finally {
    installPluginLoading.value = false;
  }
};

const handleTogglePlugin = async (plugin: Plugin) => {
  try {
    await togglePlugin(plugin.id);
    await queryClient.invalidateQueries({ queryKey: ["plugins"] });
  } catch (error) {
    console.error("切换插件状态失败:", error);
    await showInfoDialog("操作失败", "切换插件状态失败");
  }
};

const configurePlugin = async (_plugin: Plugin) => {
  await showInfoDialog("提示", "插件配置 UI 将在后续版本补齐");
};

const addOperationRecord = (operation: string, success: boolean, result: string) => {
  batchOperationHistory.value.unshift({
    operation,
    timestamp: new Date().toLocaleString("zh-CN"),
    success,
    result,
  });
};

const handleBatchDeleteArchives = async () => {
  const confirmed = await askForConfirmation({
    title: "确认批量删除",
    message: "确定要执行批量删除漫画操作吗？此操作不可撤销！",
    type: "danger",
    confirmText: "确认删除",
  });

  if (!confirmed) return;

  batchOperationLoading.value = true;
  try {
    const archiveIds = batchDeleteForm.value.archiveIds
      ? batchDeleteForm.value.archiveIds.split(",").map((id) => id.trim())
      : [];

    await batchDeleteArchives(archiveIds);
    addOperationRecord("批量删除漫画", true, `删除了 ${archiveIds.length || "所有"} 个漫画`);
    batchDeleteForm.value.archiveIds = "";
    await queryClient.invalidateQueries({ queryKey: ["archives"] });
  } catch (error) {
    console.error("批量删除漫画失败:", error);
    addOperationRecord("批量删除漫画", false, (error as Error).message);
  } finally {
    batchOperationLoading.value = false;
  }
};

const handleBatchDeleteCategoryArchives = async () => {
  const confirmed = await askForConfirmation({
    title: "确认按分类删除",
    message: "确定要删除该分类下的所有漫画吗？此操作不可撤销！",
    type: "danger",
    confirmText: "确认删除",
  });

  if (!confirmed) return;

  batchOperationLoading.value = true;
  try {
    const categoryId = batchDeleteForm.value.categoryId;
    await batchDeleteCategoryArchives(categoryId);

    const categoryName = categories.value.find((c) => c.id === categoryId)?.name || categoryId;
    addOperationRecord("按分类删除漫画", true, `删除了分类 \"${categoryName}\" 下的所有漫画`);
    batchDeleteForm.value.categoryId = "";

    await queryClient.invalidateQueries({ queryKey: ["archives"] });
    await queryClient.invalidateQueries({ queryKey: ["categories"] });
  } catch (error) {
    console.error("按分类删除漫画失败:", error);
    addOperationRecord("按分类删除漫画", false, (error as Error).message);
  } finally {
    batchOperationLoading.value = false;
  }
};

const handleBatchDeleteTagArchives = async () => {
  const confirmed = await askForConfirmation({
    title: "确认按标签删除",
    message: "确定要删除该标签下的所有漫画吗？此操作不可撤销！",
    type: "danger",
    confirmText: "确认删除",
  });

  if (!confirmed) return;

  batchOperationLoading.value = true;
  try {
    const tagId = batchDeleteForm.value.tagId;
    await batchDeleteTagArchives(tagId);

    const tag = tags.value.find((t) => t.id === tagId);
    const tagName = tag ? `${tag.namespace}:${tag.name}` : tagId;
    addOperationRecord("按标签删除漫画", true, `删除了标签 \"${tagName}\" 下的所有漫画`);
    batchDeleteForm.value.tagId = "";

    await queryClient.invalidateQueries({ queryKey: ["archives"] });
    await queryClient.invalidateQueries({ queryKey: ["tags"] });
  } catch (error) {
    console.error("按标签删除漫画失败:", error);
    addOperationRecord("按标签删除漫画", false, (error as Error).message);
  } finally {
    batchOperationLoading.value = false;
  }
};

const handlePruneTags = async () => {
  const confirmed = await askForConfirmation({
    title: "确认清理标签",
    message: "确定要清理所有无用标签吗？",
    type: "warning",
    confirmText: "确认清理",
  });

  if (!confirmed) return;

  batchOperationLoading.value = true;
  try {
    await pruneTags();
    addOperationRecord("清理无用标签", true, "成功清理了所有未使用标签");
    await queryClient.invalidateQueries({ queryKey: ["tags"] });
  } catch (error) {
    console.error("清理标签失败:", error);
    addOperationRecord("清理无用标签", false, (error as Error).message);
  } finally {
    batchOperationLoading.value = false;
  }
};

const handlePruneCategories = async () => {
  const confirmed = await askForConfirmation({
    title: "确认清理分类",
    message: "确定要清理所有空分类吗？",
    type: "warning",
    confirmText: "确认清理",
  });

  if (!confirmed) return;

  batchOperationLoading.value = true;
  try {
    await pruneCategories();
    addOperationRecord("清理空分类", true, "成功清理了所有空分类");
    await queryClient.invalidateQueries({ queryKey: ["categories"] });
  } catch (error) {
    console.error("清理分类失败:", error);
    addOperationRecord("清理空分类", false, (error as Error).message);
  } finally {
    batchOperationLoading.value = false;
  }
};

const handleManualScan = async () => {
  scanLoading.value = true;
  scanResult.value = null;

  try {
    const result = await triggerScan();
    scanResult.value = {
      success: true,
      message: result.message,
    };
    await queryClient.invalidateQueries({ queryKey: ["archives"] });
  } catch (error) {
    console.error("手动扫描失败:", error);
    scanResult.value = {
      success: false,
      message: "扫描失败，请检查漫画库路径是否正确",
    };
  } finally {
    scanLoading.value = false;
  }
};

const saveScanSettings = async () => {
  systemLoading.value = true;

  try {
    const result = await updateScanSettings(scanSettings.value);
    scanResult.value = {
      success: true,
      message: `扫描设置已更新，实时监控状态: ${result.monitoring_status ? "已启用" : "已禁用"}`,
    };
    await showInfoDialog("操作成功", "扫描策略已保存");
  } catch (error) {
    console.error("保存扫描设置失败:", error);
    scanResult.value = {
      success: false,
      message: "保存扫描设置失败，请稍后重试",
    };
    await showInfoDialog("操作失败", "保存扫描策略失败");
  } finally {
    systemLoading.value = false;
  }
};

const loadAdminData = async () => {
  try {
    const settings = await getSettings();
    systemSettings.value = {
      ...settings,
      maxFileSize: Math.round(settings.maxFileSize / (1024 * 1024)),
    };

    cacheSettings.value = {
      cachePath: settings.imageCachePath || "",
      maxSize: settings.imageCacheSize / (1024 * 1024 * 1024),
      quality: settings.imageCacheQuality || 85,
      format: settings.imageCacheFormat || "WebP",
      strategy: "balanced",
      customConfig: {
        maxMemoryMb: 512,
        maxCachedArchives: 30,
        cacheTtlHours: 24,
        preloadPrevPages: 2,
        preloadNextPages: 3,
      },
    };
  } catch (error) {
    console.error("加载设置失败:", error);
  }

  try {
    aiSettings.value = await getAISettings();
  } catch (error) {
    console.error("加载AI设置失败:", error);
  }

  try {
    const scanConfig = await getScanSettings();
    if (scanConfig && scanConfig.scanSettings) {
      scanSettings.value = scanConfig.scanSettings;
    }
  } catch (error) {
    console.error("加载扫描设置失败:", error);
  }

  await loadCacheStatus();
};

onMounted(async () => {
  if (!isAdminSettingsRoute.value) return;
  await loadAdminData();
});

watch(isAdminSettingsRoute, (isAdmin, wasAdmin) => {
  if (isAdmin && !wasAdmin) {
    void loadAdminData();
  }
});
</script>

<style scoped>
.settings-view {
  min-height: calc(100vh - 4rem);
}
</style>
