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
              :plugin-details="pluginDetails"
              @install-plugin="showInstallPluginModal = true"
              @toggle-plugin="handleTogglePlugin"
              @configure-plugin="configurePlugin"
              @view-plugin-executions="openPluginExecutions"
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

      <BaseModal
        :show="showPluginConfigModal"
        :title="pluginConfigModalTitle"
        width="xl"
        max-height="full"
        :z-index="9999"
        @close="closePluginConfigModal"
      >
        <form class="space-y-4" @submit.prevent="savePluginConfig">
          <div v-if="pluginConfigLoading" class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-4 text-sm text-[var(--text-secondary)]">
            正在加载插件配置 schema...
          </div>

          <template v-else>
            <div
              v-if="pluginConfigSchemaFields.length === 0"
              class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-4 text-sm text-[var(--text-secondary)]"
            >
              该插件未声明可配置参数，可直接保存当前配置。
            </div>

            <div
              v-for="field in pluginConfigSchemaFields"
              :key="field.key"
              class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-4"
            >
              <label class="mb-2 block text-sm font-medium text-[var(--text-primary)]">
                {{ field.schema.title || field.key }}
                <span v-if="field.required" class="ml-0.5 text-red-500">*</span>
              </label>
              <p v-if="field.schema.description" class="mb-2 text-xs text-[var(--text-secondary)]">
                {{ field.schema.description }}
              </p>

              <select
                v-if="field.schema.enum && field.schema.enum.length > 0"
                :value="String(pluginConfigFormData[field.key] ?? '')"
                class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-card)] px-3 py-2 text-sm text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]/30"
                @change="onSchemaInputEvent(field.key, field.schema, $event)"
              >
                <option v-for="option in getSchemaEnumOptions(field.schema.enum)" :key="option" :value="option">
                  {{ option }}
                </option>
              </select>

              <label
                v-else-if="field.schema.type === 'boolean'"
                class="inline-flex items-center gap-2 text-sm text-[var(--text-primary)]"
              >
                <input
                  type="checkbox"
                  class="rounded"
                  :checked="Boolean(pluginConfigFormData[field.key])"
                  @change="onSchemaBooleanEvent(field.key, $event)"
                />
                启用
              </label>

              <input
                v-else-if="field.schema.type === 'integer' || field.schema.type === 'number'"
                type="number"
                :value="toNumberInputValue(pluginConfigFormData[field.key])"
                :step="field.schema.type === 'integer' ? 1 : 'any'"
                :min="field.schema.minimum"
                :max="field.schema.maximum"
                class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-card)] px-3 py-2 text-sm text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]/30"
                @input="onSchemaInputEvent(field.key, field.schema, $event)"
              />

              <textarea
                v-else-if="field.schema.type === 'object'"
                :value="pluginConfigFieldDrafts[field.key] ?? ''"
                rows="4"
                class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-card)] px-3 py-2 text-sm text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]/30"
                @input="onSchemaObjectDraftEvent(field.key, $event)"
                @blur="parseSchemaObjectDraft(field.key)"
              />

              <input
                v-else-if="field.schema.type === 'array'"
                type="text"
                :value="pluginConfigFieldDrafts[field.key] ?? ''"
                class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-card)] px-3 py-2 text-sm text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]/30"
                @input="onSchemaArrayDraftEvent(field.key, $event)"
              />

              <input
                v-else
                type="text"
                :value="String(pluginConfigFormData[field.key] ?? '')"
                class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-card)] px-3 py-2 text-sm text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]/30"
                @input="onSchemaInputEvent(field.key, field.schema, $event)"
              />

              <p v-if="pluginConfigFieldErrors[field.key]" class="mt-2 text-xs text-red-500">
                {{ pluginConfigFieldErrors[field.key] }}
              </p>
            </div>

            <div class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-4">
              <div class="mb-2 text-sm font-medium text-[var(--text-primary)]">权限声明</div>
              <div class="flex flex-wrap gap-2">
                <span
                  v-for="permission in pluginConfigPermissionSummary"
                  :key="permission"
                  class="rounded-full border border-[var(--border)] bg-[var(--bg-card)] px-2 py-1 text-xs text-[var(--text-secondary)]"
                >
                  {{ permission }}
                </span>
              </div>
            </div>

            <p v-if="pluginConfigFormError" class="text-sm text-red-500">{{ pluginConfigFormError }}</p>
          </template>

          <div class="flex justify-end gap-2">
            <GlassButton variant="ghost" @click="closePluginConfigModal">取消</GlassButton>
            <GlassButton
              type="submit"
              variant="primary"
              :loading="pluginConfigSaving"
              loading-text="保存中..."
              :disabled="pluginConfigLoading"
            >
              保存配置
            </GlassButton>
          </div>
        </form>
      </BaseModal>

      <BaseModal
        :show="showPluginExecutionsModal"
        :title="pluginExecutionsModalTitle"
        width="2xl"
        max-height="full"
        :z-index="9999"
        @close="closePluginExecutionsModal"
      >
        <div class="space-y-4">
          <div class="flex flex-wrap items-center justify-between gap-2">
            <div class="text-xs text-[var(--text-secondary)]">共 {{ pluginExecutionTotal }} 条记录</div>
            <div class="flex items-center gap-2">
              <select
                v-model="pluginExecutionStatusFilter"
                class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-sm text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]/30"
                @change="reloadPluginExecutions"
              >
                <option value="all">全部状态</option>
                <option value="pending">待执行</option>
                <option value="running">执行中</option>
                <option value="success">成功</option>
                <option value="failed">失败</option>
              </select>
              <GlassButton size="sm" variant="secondary" :loading="pluginExecutionLoading" @click="reloadPluginExecutions">
                刷新
              </GlassButton>
            </div>
          </div>

          <div v-if="pluginExecutionLoading" class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-4 text-sm text-[var(--text-secondary)]">
            正在加载执行记录...
          </div>

          <div
            v-else-if="pluginExecutionRecords.length === 0"
            class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-4 text-sm text-[var(--text-secondary)]"
          >
            暂无执行记录
          </div>

          <div v-else class="space-y-3">
            <div
              v-for="record in pluginExecutionRecords"
              :key="record.executionId"
              class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-4"
            >
              <div class="flex flex-wrap items-center justify-between gap-2">
                <div class="text-sm font-medium text-[var(--text-primary)]">{{ record.executionId }}</div>
                <span
                  class="rounded-full px-2 py-0.5 text-xs"
                  :class="executionStatusClass(record.status)"
                >
                  {{ executionStatusLabel(record.status) }}
                </span>
              </div>
              <div class="mt-2 grid grid-cols-1 gap-2 text-xs text-[var(--text-secondary)] sm:grid-cols-2">
                <div>开始时间: {{ formatDateTime(record.startedAt) }}</div>
                <div>耗时: {{ record.durationMs !== null ? `${record.durationMs} ms` : "-" }}</div>
                <div>执行类型: {{ record.executionType || "-" }}</div>
                <div>档案: {{ record.archiveId || "全局" }}</div>
              </div>
              <p v-if="record.inputSummary" class="mt-2 text-xs text-[var(--text-secondary)]">输入: {{ record.inputSummary }}</p>
              <p v-if="record.outputSummary" class="mt-1 text-xs text-[var(--text-secondary)]">输出: {{ record.outputSummary }}</p>
              <p v-if="record.errorMessage" class="mt-1 text-xs text-red-500">错误: {{ record.errorMessage }}</p>
            </div>
          </div>
        </div>
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
import * as apiUtil from "@/utils/api";
import {
  batchDeleteArchives,
  batchDeleteCategoryArchives,
  batchDeleteTagArchives,
  clearCache as apiClearCache,
  configureCache,
  configurePlugin as configurePluginApi,
  createUser,
  deleteUser,
  getAISettings,
  getAIStatus,
  getCacheStatus,
  getCategories,
  getCategoryDeletePreview,
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
import { getApiErrorMessage } from "@/utils/error";
import type { CacheClearScope } from "@/utils/api";
import type { AISettings, Plugin, ScanSettings, SystemSettings, User } from "@/types/api";
import type {
  BatchDeleteForm,
  BatchOperationRecord,
  CacheSettingsForm,
  CacheStatusResponse,
} from "@/types/settings";

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
const cacheStatus = ref<CacheStatusResponse | null>(null);
const clearingCacheScope = ref<CacheClearScope | null>(null);
const isClearingCache = computed(() => clearingCacheScope.value !== null);

const showCreateUserModal = ref(false);
const createUserForm = ref({ username: "", email: "", password: "" });
const createUserLoading = ref(false);

const showInstallPluginModal = ref(false);
const pluginFileInput = ref<HTMLInputElement>();
const installPluginLoading = ref(false);

type JsonObject = Record<string, unknown>;

interface PluginDetailSnapshot {
  type?: string;
  permissions?: unknown;
  executionCount?: number;
  lastExecutedAt?: string | null;
  config?: JsonObject;
}

interface PluginSchemaField {
  type?: string;
  title?: string;
  description?: string;
  default?: unknown;
  enum?: unknown[];
  minimum?: number;
  maximum?: number;
}

interface PluginSchema {
  type?: string;
  properties?: Record<string, PluginSchemaField>;
  required?: string[];
}

interface PluginSchemaFieldView {
  key: string;
  required: boolean;
  schema: PluginSchemaField;
}

interface PluginExecutionRecordView {
  executionId: string;
  pluginId: string;
  archiveId: string | null;
  executionType: string;
  status: string;
  inputSummary: string | null;
  outputSummary: string | null;
  errorMessage: string | null;
  durationMs: number | null;
  startedAt: string;
  completedAt: string | null;
}

const pluginDetails = ref<Record<string, PluginDetailSnapshot>>({});
const pluginDetailLoadingIds = new Set<string>();

const showPluginConfigModal = ref(false);
const pluginConfigLoading = ref(false);
const pluginConfigSaving = ref(false);
const pluginConfigTarget = ref<Plugin | null>(null);
const pluginConfigSchema = ref<PluginSchema | null>(null);
const pluginConfigFormData = ref<Record<string, unknown>>({});
const pluginConfigFieldDrafts = ref<Record<string, string>>({});
const pluginConfigFieldErrors = ref<Record<string, string>>({});
const pluginConfigFormError = ref("");

const showPluginExecutionsModal = ref(false);
const pluginExecutionLoading = ref(false);
const pluginExecutionStatusFilter = ref("all");
const pluginExecutionTarget = ref<Plugin | null>(null);
const pluginExecutionRecords = ref<PluginExecutionRecordView[]>([]);
const pluginExecutionTotal = ref(0);

const batchOperationLoading = ref(false);
const batchDeleteForm = ref<BatchDeleteForm>({
  archiveIds: "",
  categoryId: "",
  tagId: "",
});
const batchOperationHistory = ref<BatchOperationRecord[]>([]);

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

interface SettingsActionOptions {
  logLabel: string;
  fallbackErrorMessage: string;
  successMessage?: string;
  successTitle?: string;
}

const runSettingsAction = async <T>(
  action: () => Promise<T>,
  options: SettingsActionOptions,
): Promise<T | null> => {
  try {
    const result = await action();
    if (options.successMessage) {
      await showInfoDialog(options.successTitle ?? "操作成功", options.successMessage);
    }
    return result;
  } catch (error) {
    console.error(options.logLabel, error);
    await showInfoDialog("操作失败", getApiErrorMessage(error, options.fallbackErrorMessage));
    return null;
  }
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

const normalizePluginList = (payload: unknown): Plugin[] => {
  if (Array.isArray(payload)) {
    return payload as Plugin[];
  }

  if (payload && typeof payload === "object") {
    const rawData = (payload as Record<string, unknown>).data;
    if (Array.isArray(rawData)) {
      return rawData as Plugin[];
    }
  }

  return [];
};

const users = computed(() => usersQuery.data.value ?? []);
const usersLoading = computed(() => usersQuery.isLoading.value);
const plugins = computed(() => normalizePluginList(pluginsQuery.data.value));
const pluginsLoading = computed(() => pluginsQuery.isLoading.value);
const aiStatus = computed(() => aiStatusQuery.data.value);
const categories = computed(() => categoriesQuery.data.value ?? []);
const tags = computed(() => tagsQuery.data.value ?? []);

const pluginApi = apiUtil as Record<string, unknown>;

const asObject = (value: unknown): JsonObject | null => {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return null;
  }
  return value as JsonObject;
};

const getStringFromObject = (source: JsonObject | null, keys: string[]): string | undefined => {
  if (!source) return undefined;
  for (const key of keys) {
    const value = source[key];
    if (typeof value === "string" && value.trim().length > 0) {
      return value;
    }
  }
  return undefined;
};

const getNumberFromObject = (source: JsonObject | null, keys: string[]): number | undefined => {
  if (!source) return undefined;
  for (const key of keys) {
    const value = source[key];
    if (typeof value === "number" && Number.isFinite(value)) {
      return value;
    }
  }
  return undefined;
};

const cloneValue = <T>(value: T): T => {
  if (value === null || typeof value !== "object") {
    return value;
  }
  return JSON.parse(JSON.stringify(value)) as T;
};

const resolvePluginId = (plugin: Plugin): string => {
  const rawPlugin = plugin as Plugin & Record<string, unknown>;
  const rawId = rawPlugin.id ?? rawPlugin.plugin_id;
  return typeof rawId === "string" ? rawId : "";
};

const resolvePluginName = (plugin: Plugin): string => {
  const rawPlugin = plugin as Plugin & Record<string, unknown>;
  const rawName = rawPlugin.name ?? rawPlugin.plugin_name;
  return typeof rawName === "string" && rawName.length > 0 ? rawName : "插件";
};

const mergePluginDetail = (pluginId: string, patch: PluginDetailSnapshot) => {
  if (!pluginId) return;
  const previous = pluginDetails.value[pluginId] ?? {};
  pluginDetails.value = {
    ...pluginDetails.value,
    [pluginId]: { ...previous, ...patch },
  };
};

const syncPluginSnapshotFromList = (plugin: Plugin) => {
  const pluginId = resolvePluginId(plugin);
  if (!pluginId) return;

  const rawPlugin = plugin as Plugin & Record<string, unknown>;
  const snapshot: PluginDetailSnapshot = {};
  const pluginType = getStringFromObject(rawPlugin, ["plugin_type", "type"]);
  if (pluginType) snapshot.type = pluginType;
  if (rawPlugin.permissions !== undefined) snapshot.permissions = rawPlugin.permissions;

  const executionCount = getNumberFromObject(rawPlugin, ["execution_count", "executionCount"]);
  if (executionCount !== undefined) snapshot.executionCount = executionCount;

  const lastExecutedAt = getStringFromObject(rawPlugin, ["last_executed_at", "lastExecutedAt"]);
  if (lastExecutedAt) snapshot.lastExecutedAt = lastExecutedAt;

  const rawConfig = asObject(rawPlugin.config);
  if (rawConfig) snapshot.config = cloneValue(rawConfig);

  mergePluginDetail(pluginId, snapshot);
};

const syncPluginSnapshotsFromList = (pluginList: Plugin[]) => {
  for (const plugin of pluginList) {
    syncPluginSnapshotFromList(plugin);
  }
};

const callPluginApiMethod = async <T>(methodNames: string[], ...args: unknown[]): Promise<T | null> => {
  for (const methodName of methodNames) {
    const method = pluginApi[methodName];
    if (typeof method === "function") {
      return await (method as (...params: unknown[]) => Promise<T>)(...args);
    }
  }
  return null;
};

const updatePluginDetailFromPayload = (pluginId: string, payload: unknown): JsonObject | null => {
  const detail = asObject(payload);
  if (!detail) return null;

  const patch: PluginDetailSnapshot = {};
  const pluginType = getStringFromObject(detail, ["plugin_type", "type"]);
  if (pluginType) patch.type = pluginType;
  if (detail.permissions !== undefined) patch.permissions = detail.permissions;

  const executionCount = getNumberFromObject(detail, ["execution_count", "executionCount"]);
  if (executionCount !== undefined) patch.executionCount = executionCount;

  const lastExecutedAt = getStringFromObject(detail, ["last_executed_at", "lastExecutedAt"]);
  if (lastExecutedAt) patch.lastExecutedAt = lastExecutedAt;

  const config = asObject(detail.config);
  if (config) patch.config = cloneValue(config);

  mergePluginDetail(pluginId, patch);
  return detail;
};

const fetchPluginDetail = async (pluginId: string): Promise<JsonObject | null> => {
  if (!pluginId || pluginDetailLoadingIds.has(pluginId)) {
    return null;
  }

  pluginDetailLoadingIds.add(pluginId);
  try {
    const detail = await callPluginApiMethod<unknown>(["getPluginDetail", "getPlugin"], pluginId);
    if (detail === null) return null;
    return updatePluginDetailFromPayload(pluginId, detail);
  } catch (error) {
    console.error("获取插件详情失败:", error);
    return null;
  } finally {
    pluginDetailLoadingIds.delete(pluginId);
  }
};

const hydratePluginDetails = async (pluginList: Plugin[]) => {
  if (pluginList.length === 0) return;

  const hasDetailApi =
    typeof pluginApi.getPluginDetail === "function" ||
    typeof pluginApi.getPlugin === "function";
  if (!hasDetailApi) return;

  await Promise.all(
    pluginList.map(async (plugin) => {
      const pluginId = resolvePluginId(plugin);
      if (!pluginId) return;
      const cached = pluginDetails.value[pluginId];
      if (cached?.permissions !== undefined && cached.type) return;
      await fetchPluginDetail(pluginId);
    }),
  );
};

watch(
  plugins,
  (pluginList) => {
    syncPluginSnapshotsFromList(pluginList);
    if (isAdminSettingsRoute.value && activeTab.value === "plugins") {
      void hydratePluginDetails(pluginList);
    }
  },
  { immediate: true },
);

watch(
  () => activeTab.value,
  (tab) => {
    if (tab === "plugins") {
      syncPluginSnapshotsFromList(plugins.value);
      void hydratePluginDetails(plugins.value);
    }
  },
);

const toStringArray = (value: unknown): string[] => {
  if (!Array.isArray(value)) return [];
  return value.filter((item): item is string => typeof item === "string" && item.length > 0);
};

const formatPermissionSummary = (permissions: unknown): string[] => {
  if (!permissions || typeof permissions !== "object") {
    return ["未声明权限"];
  }

  const permissionData = permissions as Record<string, unknown>;
  const summary: string[] = [];

  const networkList = toStringArray(permissionData.network);
  const hasNetwork =
    (typeof permissionData.network === "boolean" && permissionData.network) ||
    networkList.length > 0;
  summary.push(hasNetwork ? `网络(${networkList.length || "开放"})` : "无网络");

  const fsRead = toStringArray(permissionData.filesystem_read ?? permissionData.filesystemRead);
  if (fsRead.length > 0) summary.push(`读文件(${fsRead.length})`);

  const fsWrite = toStringArray(permissionData.filesystem_write ?? permissionData.filesystemWrite);
  if (fsWrite.length > 0) summary.push(`写文件(${fsWrite.length})`);

  const dbRead = permissionData.database_read ?? permissionData.databaseRead;
  if (typeof dbRead === "boolean") {
    summary.push(dbRead ? "数据库读" : "无数据库读");
  }

  const dbWrite = toStringArray(permissionData.database_write ?? permissionData.databaseWrite);
  if (dbWrite.length > 0) summary.push(`数据库写(${dbWrite.length})`);

  return summary;
};

const normalizeSchemaField = (value: unknown): PluginSchemaField => {
  const field = asObject(value);
  if (!field) return {};

  return {
    type: getStringFromObject(field, ["type"]),
    title: getStringFromObject(field, ["title"]),
    description: getStringFromObject(field, ["description"]),
    default: field.default,
    enum: Array.isArray(field.enum) ? field.enum : undefined,
    minimum: typeof field.minimum === "number" ? field.minimum : undefined,
    maximum: typeof field.maximum === "number" ? field.maximum : undefined,
  };
};

const normalizePluginSchema = (value: unknown): PluginSchema | null => {
  const rawSchema = asObject(value);
  if (!rawSchema) return null;

  const rawProperties = asObject(rawSchema.properties);
  const properties: Record<string, PluginSchemaField> = {};
  if (rawProperties) {
    for (const [key, field] of Object.entries(rawProperties)) {
      properties[key] = normalizeSchemaField(field);
    }
  }

  const required = Array.isArray(rawSchema.required)
    ? rawSchema.required.filter((item): item is string => typeof item === "string")
    : [];

  return {
    type: getStringFromObject(rawSchema, ["type"]),
    properties,
    required,
  };
};

const extractConfigSchema = (payload: unknown): PluginSchema | null => {
  const fromRoot = normalizePluginSchema(payload);
  if (fromRoot?.properties && Object.keys(fromRoot.properties).length > 0) {
    return fromRoot;
  }

  const raw = asObject(payload);
  if (!raw) return fromRoot;

  return normalizePluginSchema(raw.config_schema ?? raw.configSchema);
};

const defaultValueByType = (field: PluginSchemaField): unknown => {
  if (field.default !== undefined) {
    return cloneValue(field.default);
  }

  if (field.enum && field.enum.length > 0) {
    return cloneValue(field.enum[0]);
  }

  switch (field.type) {
    case "boolean":
      return false;
    case "integer":
    case "number":
      return 0;
    case "array":
      return [];
    case "object":
      return {};
    default:
      return "";
  }
};

const initializePluginConfigData = (
  schema: PluginSchema | null,
  currentConfig: Record<string, unknown>,
): Record<string, unknown> => {
  const nextConfig: Record<string, unknown> = { ...currentConfig };
  const properties = schema?.properties ?? {};

  for (const [key, field] of Object.entries(properties)) {
    if (nextConfig[key] === undefined) {
      nextConfig[key] = defaultValueByType(field);
    }
  }

  return nextConfig;
};

const createConfigFieldDrafts = (
  schema: PluginSchema | null,
  config: Record<string, unknown>,
): Record<string, string> => {
  const drafts: Record<string, string> = {};
  const properties = schema?.properties ?? {};

  for (const [key, field] of Object.entries(properties)) {
    const value = config[key];
    if (field.type === "array") {
      drafts[key] = Array.isArray(value) ? value.join(", ") : "";
    }
    if (field.type === "object") {
      drafts[key] = value ? JSON.stringify(value, null, 2) : "{}";
    }
  }

  return drafts;
};

const pluginConfigModalTitle = computed(() => {
  const target = pluginConfigTarget.value as Plugin | null;
  if (!target) return "插件配置";
  return `${resolvePluginName(target)} 配置`;
});

const pluginConfigSchemaFields = computed<PluginSchemaFieldView[]>(() => {
  const schema = pluginConfigSchema.value;
  if (!schema?.properties) return [];
  const requiredSet = new Set(schema.required ?? []);

  return Object.entries(schema.properties).map(([key, field]) => ({
    key,
    schema: field,
    required: requiredSet.has(key),
  }));
});

const pluginConfigPermissionSummary = computed(() => {
  const target = pluginConfigTarget.value as Plugin | null;
  if (!target) return ["未声明权限"];
  const pluginId = resolvePluginId(target);
  return formatPermissionSummary(pluginDetails.value[pluginId]?.permissions);
});

const getSchemaEnumOptions = (enumValues: unknown[] | undefined): string[] => {
  if (!enumValues) return [];
  return enumValues.map((option) => String(option));
};

const toNumberInputValue = (value: unknown): string => {
  return typeof value === "number" && Number.isFinite(value) ? String(value) : "";
};

const setPluginConfigField = (key: string, value: unknown) => {
  pluginConfigFieldErrors.value = {
    ...pluginConfigFieldErrors.value,
    [key]: "",
  };
  pluginConfigFormData.value = {
    ...pluginConfigFormData.value,
    [key]: value,
  };
};

const setSchemaFieldBoolean = (key: string, checked: boolean) => {
  setPluginConfigField(key, checked);
};

const setSchemaFieldFromInput = (key: string, schema: PluginSchemaField, rawValue: string) => {
  switch (schema.type) {
    case "integer": {
      if (!rawValue.trim()) {
        setPluginConfigField(key, 0);
        return;
      }
      const parsed = Number.parseInt(rawValue, 10);
      setPluginConfigField(key, Number.isFinite(parsed) ? parsed : 0);
      return;
    }
    case "number": {
      if (!rawValue.trim()) {
        setPluginConfigField(key, 0);
        return;
      }
      const parsed = Number(rawValue);
      setPluginConfigField(key, Number.isFinite(parsed) ? parsed : 0);
      return;
    }
    default:
      setPluginConfigField(key, rawValue);
  }
};

const getEventTargetValue = (event: Event): string => {
  const target = event.target;
  if (
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    target instanceof HTMLSelectElement
  ) {
    return target.value;
  }
  return "";
};

const getEventTargetChecked = (event: Event): boolean => {
  const target = event.target;
  return target instanceof HTMLInputElement ? target.checked : false;
};

const onSchemaInputEvent = (key: string, schema: PluginSchemaField, event: Event) => {
  setSchemaFieldFromInput(key, schema, getEventTargetValue(event));
};

const onSchemaBooleanEvent = (key: string, event: Event) => {
  setSchemaFieldBoolean(key, getEventTargetChecked(event));
};

const setSchemaArrayDraft = (key: string, rawValue: string) => {
  pluginConfigFieldDrafts.value = {
    ...pluginConfigFieldDrafts.value,
    [key]: rawValue,
  };
  const parsed = rawValue
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
  setPluginConfigField(key, parsed);
};

const onSchemaArrayDraftEvent = (key: string, event: Event) => {
  setSchemaArrayDraft(key, getEventTargetValue(event));
};

const setSchemaObjectDraft = (key: string, rawValue: string) => {
  pluginConfigFieldDrafts.value = {
    ...pluginConfigFieldDrafts.value,
    [key]: rawValue,
  };
};

const onSchemaObjectDraftEvent = (key: string, event: Event) => {
  setSchemaObjectDraft(key, getEventTargetValue(event));
};

const parseSchemaObjectDraft = (key: string): boolean => {
  const draft = pluginConfigFieldDrafts.value[key];
  if (draft === undefined || draft.trim() === "") {
    setPluginConfigField(key, {});
    return true;
  }

  try {
    const parsed = JSON.parse(draft);
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
      pluginConfigFieldErrors.value = {
        ...pluginConfigFieldErrors.value,
        [key]: "必须是 JSON 对象",
      };
      return false;
    }
    setPluginConfigField(key, parsed);
    return true;
  } catch {
    pluginConfigFieldErrors.value = {
      ...pluginConfigFieldErrors.value,
      [key]: "JSON 格式不正确",
    };
    return false;
  }
};

const parseObjectDraftsBeforeSave = (): boolean => {
  const objectFields = pluginConfigSchemaFields.value.filter((field) => field.schema.type === "object");
  for (const field of objectFields) {
    if (!parseSchemaObjectDraft(field.key)) {
      return false;
    }
  }
  return true;
};

const validatePluginConfig = (): boolean => {
  const fieldErrors: Record<string, string> = {};
  const requiredSet = new Set(pluginConfigSchema.value?.required ?? []);

  for (const key of requiredSet) {
    const value = pluginConfigFormData.value[key];
    const isEmptyString = typeof value === "string" && value.trim().length === 0;
    const isEmptyArray = Array.isArray(value) && value.length === 0;
    if (value === undefined || value === null || isEmptyString || isEmptyArray) {
      fieldErrors[key] = "该字段为必填项";
    }
  }

  pluginConfigFieldErrors.value = {
    ...pluginConfigFieldErrors.value,
    ...fieldErrors,
  };

  return Object.keys(fieldErrors).length === 0;
};

const closePluginConfigModal = () => {
  showPluginConfigModal.value = false;
  pluginConfigTarget.value = null;
  pluginConfigSchema.value = null;
  pluginConfigFormData.value = {};
  pluginConfigFieldDrafts.value = {};
  pluginConfigFieldErrors.value = {};
  pluginConfigFormError.value = "";
};

const openPluginConfig = async (plugin: Plugin) => {
  const pluginId = resolvePluginId(plugin);
  if (!pluginId) {
    await showInfoDialog("操作失败", "无法识别插件 ID");
    return;
  }

  showPluginConfigModal.value = true;
  pluginConfigTarget.value = plugin;
  pluginConfigLoading.value = true;
  pluginConfigFormError.value = "";
  pluginConfigFieldErrors.value = {};

  try {
    syncPluginSnapshotFromList(plugin);
    const detail = await fetchPluginDetail(pluginId);

    const schemaResponse = await callPluginApiMethod<unknown>(
      ["getPluginConfigSchema", "getPluginSchema"],
      pluginId,
    );

    const schema =
      extractConfigSchema(schemaResponse) ??
      extractConfigSchema(detail?.manifest) ??
      extractConfigSchema((plugin as unknown as Record<string, unknown>).config_schema) ??
      { type: "object", properties: {}, required: [] };

    pluginConfigSchema.value = schema;

    const configFromDetail = asObject(detail?.config);
    const configFromCache = pluginDetails.value[pluginId]?.config;
    const configFromPlugin = asObject((plugin as unknown as Record<string, unknown>).config);
    const currentConfig = configFromDetail ?? configFromCache ?? configFromPlugin ?? {};

    const initializedConfig = initializePluginConfigData(schema, currentConfig);
    pluginConfigFormData.value = initializedConfig;
    pluginConfigFieldDrafts.value = createConfigFieldDrafts(schema, initializedConfig);
  } catch (error) {
    console.error("加载插件配置失败:", error);
    pluginConfigFormError.value = getApiErrorMessage(error, "加载插件配置失败");
  } finally {
    pluginConfigLoading.value = false;
  }
};

const savePluginConfig = async () => {
  const target = pluginConfigTarget.value as Plugin | null;
  if (!target) return;

  pluginConfigFormError.value = "";
  if (!parseObjectDraftsBeforeSave() || !validatePluginConfig()) {
    pluginConfigFormError.value = "请先修正配置项中的错误";
    return;
  }

  const pluginId = resolvePluginId(target);
  if (!pluginId) return;

  pluginConfigSaving.value = true;
  try {
    const result = await runSettingsAction(
      async () => {
        await configurePluginApi(pluginId, pluginConfigFormData.value);
        await queryClient.invalidateQueries({ queryKey: ["plugins"] });
        await fetchPluginDetail(pluginId);
      },
      {
        logLabel: "保存插件配置失败:",
        fallbackErrorMessage: "保存插件配置失败",
        successMessage: "插件配置已保存",
      },
    );

    if (result !== null) {
      closePluginConfigModal();
    }
  } finally {
    pluginConfigSaving.value = false;
  }
};

const normalizeExecutionRecord = (record: unknown): PluginExecutionRecordView | null => {
  const raw = asObject(record);
  if (!raw) return null;

  const executionId = getStringFromObject(raw, ["execution_id", "executionId", "id"]) ?? "";
  if (!executionId) return null;

  return {
    executionId,
    pluginId: getStringFromObject(raw, ["plugin_id", "pluginId"]) ?? "",
    archiveId: getStringFromObject(raw, ["archive_id", "archiveId"]) ?? null,
    executionType: getStringFromObject(raw, ["execution_type", "executionType"]) ?? "",
    status: getStringFromObject(raw, ["status"]) ?? "unknown",
    inputSummary: getStringFromObject(raw, ["input_summary", "inputSummary"]) ?? null,
    outputSummary: getStringFromObject(raw, ["output_summary", "outputSummary"]) ?? null,
    errorMessage: getStringFromObject(raw, ["error_message", "errorMessage"]) ?? null,
    durationMs: getNumberFromObject(raw, ["duration_ms", "durationMs"]) ?? null,
    startedAt: getStringFromObject(raw, ["started_at", "startedAt"]) ?? "",
    completedAt: getStringFromObject(raw, ["completed_at", "completedAt"]) ?? null,
  };
};

const parseExecutionListResponse = (
  payload: unknown,
): { total: number; items: PluginExecutionRecordView[] } => {
  if (Array.isArray(payload)) {
    const items = payload
      .map((item) => normalizeExecutionRecord(item))
      .filter((item): item is PluginExecutionRecordView => item !== null);
    return { total: items.length, items };
  }

  const raw = asObject(payload);
  if (!raw) {
    return { total: 0, items: [] };
  }

  const rawItems = Array.isArray(raw.items) ? raw.items : [];
  const items = rawItems
    .map((item) => normalizeExecutionRecord(item))
    .filter((item): item is PluginExecutionRecordView => item !== null);

  const total = getNumberFromObject(raw, ["total"]) ?? items.length;
  return { total, items };
};

const pluginExecutionsModalTitle = computed(() => {
  const target = pluginExecutionTarget.value as Plugin | null;
  if (!target) return "执行记录";
  return `${resolvePluginName(target)} 执行记录`;
});

const executionStatusClass = (status: string): string => {
  switch (status) {
    case "success":
      return "bg-green-500/15 text-green-500";
    case "failed":
      return "bg-red-500/15 text-red-500";
    case "running":
      return "bg-blue-500/15 text-blue-500";
    case "pending":
      return "bg-yellow-500/15 text-yellow-600";
    default:
      return "bg-[var(--bg-card)] text-[var(--text-secondary)]";
  }
};

const executionStatusLabel = (status: string): string => {
  switch (status) {
    case "success":
      return "成功";
    case "failed":
      return "失败";
    case "running":
      return "执行中";
    case "pending":
      return "待执行";
    default:
      return status || "未知";
  }
};

const formatDateTime = (dateString: string | null | undefined): string => {
  if (!dateString) return "-";
  const date = new Date(dateString);
  if (Number.isNaN(date.getTime())) return "-";
  return date.toLocaleString("zh-CN", {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
};

const closePluginExecutionsModal = () => {
  showPluginExecutionsModal.value = false;
  pluginExecutionTarget.value = null;
  pluginExecutionRecords.value = [];
  pluginExecutionTotal.value = 0;
  pluginExecutionStatusFilter.value = "all";
};

const reloadPluginExecutions = async () => {
  const target = pluginExecutionTarget.value as Plugin | null;
  if (!target) return;

  const pluginId = resolvePluginId(target);
  if (!pluginId) return;

  pluginExecutionLoading.value = true;
  try {
    const params: Record<string, unknown> = { limit: 50 };
    if (pluginExecutionStatusFilter.value !== "all") {
      params.status = pluginExecutionStatusFilter.value;
    }

    let response = await callPluginApiMethod<unknown>(
      ["getPluginExecutions", "getPluginExecutionHistory"],
      pluginId,
      params,
    );

    if (response === null) {
      response = await callPluginApiMethod<unknown>(
        ["getAllPluginExecutions", "listPluginExecutions"],
        { ...params, plugin_id: pluginId },
      );
    }

    if (response === null) {
      pluginExecutionRecords.value = [];
      pluginExecutionTotal.value = 0;
      await showInfoDialog("提示", "当前 API util 未提供执行记录查询方法");
      return;
    }

    const parsed = parseExecutionListResponse(response);
    pluginExecutionRecords.value = parsed.items;
    pluginExecutionTotal.value = parsed.total;
  } catch (error) {
    console.error("加载插件执行记录失败:", error);
    await showInfoDialog("操作失败", getApiErrorMessage(error, "加载插件执行记录失败"));
  } finally {
    pluginExecutionLoading.value = false;
  }
};

const openPluginExecutions = async (plugin: Plugin) => {
  const pluginId = resolvePluginId(plugin);
  if (!pluginId) {
    await showInfoDialog("操作失败", "无法识别插件 ID");
    return;
  }

  showPluginExecutionsModal.value = true;
  pluginExecutionTarget.value = plugin;
  pluginExecutionRecords.value = [];
  pluginExecutionTotal.value = 0;
  await reloadPluginExecutions();
};

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

const normalizeCacheFormat = (
  format: string | undefined,
): CacheSettingsForm["format"] => {
  if (format === "JPEG" || format === "PNG" || format === "WebP") {
    return format;
  }
  return "WebP";
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
    await runSettingsAction(
      async () => {
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
          strategy:
            cacheSettings.value.strategy === "custom"
              ? undefined
              : cacheSettings.value.strategy,
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
      },
      {
        logLabel: "保存系统设置失败:",
        fallbackErrorMessage: "保存失败，请稍后重试",
        successMessage: "系统与缓存配置已保存",
      },
    );
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
    await showInfoDialog(
      "操作失败",
      getApiErrorMessage(error, `清理${scopeNameMap[scope]}失败`),
    );
  } finally {
    clearingCacheScope.value = null;
  }
};

const saveAISettings = async () => {
  aiLoading.value = true;
  try {
    await runSettingsAction(
      () => updateAISettings(aiSettings.value),
      {
        logLabel: "保存AI设置失败:",
        fallbackErrorMessage: "保存失败，请稍后重试",
        successMessage: "AI 设置已保存",
      },
    );
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
    const result = await runSettingsAction(
      async () => {
        await createUser({
          username,
          password: createUserForm.value.password,
          email: email || undefined,
        });

        await queryClient.invalidateQueries({ queryKey: ["users"] });
      },
      {
        logLabel: "创建用户失败:",
        fallbackErrorMessage: "创建用户失败",
        successMessage: "用户创建成功",
      },
    );

    if (result !== null) {
      showCreateUserModal.value = false;
      createUserForm.value = { username: "", email: "", password: "" };
    }
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

  await runSettingsAction(
    async () => {
      await deleteUser(user.id);
      await queryClient.invalidateQueries({ queryKey: ["users"] });
    },
    {
      logLabel: "删除用户失败:",
      fallbackErrorMessage: "删除用户失败",
    },
  );
};

const handleInstallPlugin = async () => {
  if (!pluginFileInput.value?.files?.[0]) return;

  installPluginLoading.value = true;
  try {
    const result = await runSettingsAction(
      async () => {
        const formData = new FormData();
        formData.append("plugin", pluginFileInput.value!.files![0]);

        await installPlugin(formData);
        await queryClient.invalidateQueries({ queryKey: ["plugins"] });
      },
      {
        logLabel: "安装插件失败:",
        fallbackErrorMessage: "安装插件失败",
        successMessage: "插件安装成功",
      },
    );

    if (result !== null) {
      showInstallPluginModal.value = false;
      if (pluginFileInput.value) {
        pluginFileInput.value.value = "";
      }
      syncPluginSnapshotsFromList(plugins.value);
    }
  } finally {
    installPluginLoading.value = false;
  }
};

const handleTogglePlugin = async (plugin: Plugin) => {
  const pluginId = resolvePluginId(plugin);
  if (!pluginId) {
    await showInfoDialog("操作失败", "无法识别插件 ID");
    return;
  }

  await runSettingsAction(
    async () => {
      await togglePlugin(pluginId);
      await queryClient.invalidateQueries({ queryKey: ["plugins"] });
      await fetchPluginDetail(pluginId);
    },
    {
      logLabel: "切换插件状态失败:",
      fallbackErrorMessage: "切换插件状态失败",
    },
  );
};

const configurePlugin = async (plugin: Plugin) => {
  await openPluginConfig(plugin);
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
    addOperationRecord("批量删除漫画", false, getApiErrorMessage(error, "批量删除漫画失败"));
  } finally {
    batchOperationLoading.value = false;
  }
};

const handleBatchDeleteCategoryArchives = async () => {
  const categoryId = batchDeleteForm.value.categoryId;
  const category = categories.value.find((item) => item.id === categoryId);
  if (!categoryId || !category) return;

  batchOperationLoading.value = true;
  try {
    const preview = await getCategoryDeletePreview(categoryId);
    if (preview.matched === 0) {
      await showInfoDialog("没有可删除内容", `分类 "${category.name}" 当前没有匹配的漫画。`);
      return;
    }

    const categoryType = preview.categoryType === "dynamic" ? "动态分类" : "静态分类";
    const confirmed = await askForConfirmation({
      title: "确认按分类删除",
      message: `${categoryType} "${category.name}" 当前匹配 ${preview.matched} 个漫画。确定永久删除这些漫画吗？`,
      type: "danger",
      confirmText: `删除 ${preview.matched} 个漫画`,
    });

    if (!confirmed) return;

    const result = await batchDeleteCategoryArchives(categoryId);
    const operationSucceeded = result.failed === 0;
    const resultMessage = `匹配 ${result.matched} 个，成功删除 ${result.deleted} 个，失败 ${result.failed} 个`;

    addOperationRecord("按分类删除漫画", operationSucceeded, resultMessage);
    batchDeleteForm.value.categoryId = "";

    await queryClient.invalidateQueries({ queryKey: ["archives"] });
    await queryClient.invalidateQueries({ queryKey: ["categories"] });

    if (!operationSucceeded) {
      await showInfoDialog("部分删除失败", resultMessage);
    }
  } catch (error) {
    console.error("按分类删除漫画失败:", error);
    const responseStatus = (error as { response?: { status?: number } })?.response?.status;
    const errorMessage = responseStatus === 422
      ? "动态分类没有有效筛选条件，已拒绝可能删除整个漫画库的操作。"
      : getApiErrorMessage(error, "按分类删除漫画失败");
    addOperationRecord("按分类删除漫画", false, errorMessage);
    await showInfoDialog("按分类删除失败", errorMessage);
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
    addOperationRecord("按标签删除漫画", false, getApiErrorMessage(error, "按标签删除漫画失败"));
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
    addOperationRecord("清理无用标签", false, getApiErrorMessage(error, "清理无用标签失败"));
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
    addOperationRecord("清理空分类", false, getApiErrorMessage(error, "清理空分类失败"));
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
    const result = await runSettingsAction(
      () => updateScanSettings(scanSettings.value),
      {
        logLabel: "保存扫描设置失败:",
        fallbackErrorMessage: "保存扫描设置失败，请稍后重试",
        successMessage: "扫描策略已保存",
      },
    );

    if (result) {
      scanResult.value = {
        success: true,
        message: `扫描设置已更新，实时监控状态: ${result.monitoring_status ? "已启用" : "已禁用"}`,
      };
    } else {
      scanResult.value = {
        success: false,
        message: "保存扫描设置失败，请稍后重试",
      };
    }
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
      format: normalizeCacheFormat(settings.imageCacheFormat),
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
    scanSettings.value = scanConfig.scanSettings;
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
