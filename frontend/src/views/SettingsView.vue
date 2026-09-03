<template>
  <BasePageView theme="settings">
    <div class="settings-view w-full p-4 sm:p-6">
      <div class="mx-auto w-full max-w-7xl">
        <GlassCard size="sm" radius="lg">
          <div class="flex items-start justify-between gap-4">
            <div>
              <h1 class="text-2xl font-bold text-[var(--text-primary)]">
                {{ pageTitle }}
              </h1>
              <p class="mt-1 text-sm text-[var(--text-secondary)]">
                {{ pageSubtitle }}
              </p>
            </div>
          </div>
        </GlassCard>

        <div
          class="mt-6 grid grid-cols-1 gap-6 lg:grid-cols-[260px_minmax(0,1fr)]"
        >
          <SettingsNav
            :items="tabs"
            :active-tab="activeTab"
            :dirty-tabs="dirtyTabs"
            @select="setActiveTab"
          />

          <div class="min-w-0 pb-20">
            <Transition name="settings-panel" mode="out-in">
              <div :key="activeTab" class="min-w-0">
                <AppearanceSettingsSection
                  v-if="isUserSettingsRoute && activeTab === 'appearance'"
                  :theme="theme"
                  :show-carousel="libraryStore.showCarousel"
                  :rows-per-page="libraryStore.rowsPerPage"
                  @change-theme="setTheme"
                  @toggle-carousel="
                    libraryStore.setShowCarousel(!libraryStore.showCarousel)
                  "
                  @change-rows="libraryStore.setRowsPerPage"
                />

                <RecommendationInsightsSection
                  v-if="isUserSettingsRoute && activeTab === 'recommendations'"
                />

                <TrashSettingsSection v-if="activeTab === 'trash'" />

                <SystemSettingsSection
                  v-if="isAdminSettingsRoute && activeTab === 'system'"
                  :system-settings="systemSettings"
                  :cache-settings="cacheSettings"
                  :scan-settings="scanSettings"
                  :cache-status="cacheStatus"
                  :cache-strategy-description="getCacheStrategyDescription()"
                  :system-loading="systemLoading"
                  :system-dirty="isSystemDirty"
                  :saved-message="systemSavedMessage"
                  :save-error="systemSaveError"
                  :scan-loading="scanLoading"
                  :scan-result="scanResult"
                  :is-clearing-cache="isClearingCache"
                  :clearing-cache-scope="clearingCacheScope"
                  @select-comics-path="selectComicsPath"
                  @select-cache-path="selectCachePath"
                  @save-system="saveSystemSettings"
                  @discard-system="discardSystemChanges"
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
                  v-if="isAdminSettingsRoute && isAIProcessingTab"
                  :section="aiSectionForTab"
                  :ai-settings="aiSettings"
                  :ai-status="aiStatus"
                  :ai-loading="isAIModelsLoading"
                  :ai-dirty="isAIDirty"
                  :save-bar-dirty="
                    activeTab === 'ai-models' ? isAIModelsDirty : undefined
                  "
                  :save-bar-saving="
                    activeTab === 'ai-models' ? aiModelSaving : undefined
                  "
                  :saved-message="aiSavedMessage"
                  :save-error="aiSaveError"
                  :testing-connection="testingAIConnection"
                  :previewing-title-translation="previewingTitleTranslation"
                  :title-translation-preview="titleTranslationPreview"
                  :backfilling-translations="backfillingTitleTranslations"
                  :repairing-translations="repairingTitleTranslations"
                  :retranslating-translations="retranslatingTitleTranslations"
                  :backfilling-tagging="backfillingAITagging"
                  :backfilling-tag-localizations="backfillingAITagLocalizations"
                  :loading-tag-suggestions="loadingAITagSuggestions"
                  :tag-suggestions="tagSuggestions"
                  :tag-suggestions-total="tagSuggestionsTotal"
                  :tag-suggestions-page="tagSuggestionPage"
                  :tag-suggestions-page-count="tagSuggestionPageCount"
                  :reviewing-tag-suggestion-id="reviewingAITagSuggestionId"
                  :undoing-tagging-run-id="undoingAITaggingRunId"
                  :recent-tagging-run-ids="recentTaggingRunIds"
                  :controlling-task-queue="controllingAITaskQueue"
                  :controlling-model="controllingAIModel"
                  @save="saveCurrentSettings"
                  @discard="discardCurrentSettings"
                  @test-connection="handleTestAIConnection"
                  @preview-title-translation="handlePreviewTitleTranslation"
                  @backfill-title-translations="handleBackfillTitleTranslations"
                  @repair-suspicious-title-translations="
                    handleRepairSuspiciousTitleTranslations
                  "
                  @force-retranslate-title-translations="
                    handleForceRetranslateTitleTranslations
                  "
                  @backfill-auto-tagging="handleBackfillAITagging"
                  @backfill-tag-localizations="handleBackfillAITagLocalizations"
                  @refresh-tag-suggestions="refreshAITagSuggestions"
                  @change-tag-suggestions-page="changeAITagSuggestionsPage"
                  @review-tag-suggestion="handleReviewAITagSuggestion"
                  @undo-tagging-run="handleUndoAITaggingRun"
                  @control-task-queue="handleControlAITaskQueue"
                  @force-continue-model="handleForceContinueAIModel"
                  @update-execution-lane="updateAIExecutionLane"
                />

                <EmbeddingSettingsSection
                  v-if="isAdminSettingsRoute && activeTab === 'ai-models'"
                  ref="embeddingSettingsRef"
                  class="mt-6"
                  :show-save-bar="false"
                  @dirty-change="embeddingDirty = $event"
                />

                <OCRSettingsSection
                  ref="ocrSettingsRef"
                  v-if="isAdminSettingsRoute && activeTab === 'ai-models'"
                  class="mt-6"
                  :show-save-controls="false"
                  @dirty-change="ocrDirty = $event"
                />
              </div>
            </Transition>
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
          <GlassInput
            v-model="createUserForm.username"
            label="用户名"
            type="text"
            required
            placeholder="输入用户名"
          />
          <GlassInput
            v-model="createUserForm.email"
            label="邮箱"
            type="email"
            placeholder="输入邮箱（可选）"
          />
          <GlassInput
            v-model="createUserForm.password"
            label="密码"
            type="password"
            required
            placeholder="输入密码"
          />

          <div class="flex justify-end gap-2">
            <GlassButton variant="ghost" @click="showCreateUserModal = false"
              >取消</GlassButton
            >
            <GlassButton
              type="submit"
              :loading="createUserLoading"
              loading-text="创建中..."
              variant="primary"
              >创建</GlassButton
            >
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
            <label
              class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
              >插件文件 (.zip)</label
            >
            <input
              ref="pluginFileInput"
              type="file"
              accept=".zip"
              required
              class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-sm text-[var(--text-primary)] focus:border-[var(--accent)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]/30"
            />
          </div>

          <div class="flex justify-end gap-2">
            <GlassButton variant="ghost" @click="showInstallPluginModal = false"
              >取消</GlassButton
            >
            <GlassButton
              type="submit"
              :loading="installPluginLoading"
              loading-text="安装中..."
              variant="primary"
              >安装</GlassButton
            >
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
          <div
            v-if="pluginConfigLoading"
            class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-4 text-sm text-[var(--text-secondary)]"
          >
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
              <label
                class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
              >
                {{ field.schema.title || field.key }}
                <span v-if="field.required" class="ml-0.5 text-red-500">*</span>
              </label>
              <p
                v-if="field.schema.description"
                class="mb-2 text-xs text-[var(--text-secondary)]"
              >
                {{ field.schema.description }}
              </p>

              <select
                v-if="field.schema.enum && field.schema.enum.length > 0"
                :value="String(pluginConfigFormData[field.key] ?? '')"
                class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-card)] px-3 py-2 text-sm text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]/30"
                @change="onSchemaInputEvent(field.key, field.schema, $event)"
              >
                <option
                  v-for="option in getSchemaEnumOptions(field.schema.enum)"
                  :key="option"
                  :value="option"
                >
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
                v-else-if="
                  field.schema.type === 'integer' ||
                  field.schema.type === 'number'
                "
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

              <p
                v-if="pluginConfigFieldErrors[field.key]"
                class="mt-2 text-xs text-red-500"
              >
                {{ pluginConfigFieldErrors[field.key] }}
              </p>
            </div>

            <div
              class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-4"
            >
              <div class="mb-2 text-sm font-medium text-[var(--text-primary)]">
                权限声明
              </div>
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

            <p v-if="pluginConfigFormError" class="text-sm text-red-500">
              {{ pluginConfigFormError }}
            </p>
          </template>

          <div class="flex justify-end gap-2">
            <GlassButton variant="ghost" @click="closePluginConfigModal"
              >取消</GlassButton
            >
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
            <div class="text-xs text-[var(--text-secondary)]">
              共 {{ pluginExecutionTotal }} 条记录
            </div>
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
              <GlassButton
                size="sm"
                variant="secondary"
                :loading="pluginExecutionLoading"
                @click="reloadPluginExecutions"
              >
                刷新
              </GlassButton>
            </div>
          </div>

          <div
            v-if="pluginExecutionLoading"
            class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-4 text-sm text-[var(--text-secondary)]"
          >
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
                <div class="text-sm font-medium text-[var(--text-primary)]">
                  {{ record.executionId }}
                </div>
                <span
                  class="rounded-full px-2 py-0.5 text-xs"
                  :class="executionStatusClass(record.status)"
                >
                  {{ executionStatusLabel(record.status) }}
                </span>
              </div>
              <div
                class="mt-2 grid grid-cols-1 gap-2 text-xs text-[var(--text-secondary)] sm:grid-cols-2"
              >
                <div>开始时间: {{ formatDateTime(record.startedAt) }}</div>
                <div>
                  耗时:
                  {{
                    record.durationMs !== null ? `${record.durationMs} ms` : "-"
                  }}
                </div>
                <div>执行类型: {{ record.executionType || "-" }}</div>
                <div>档案: {{ record.archiveId || "全局" }}</div>
              </div>
              <p
                v-if="record.inputSummary"
                class="mt-2 text-xs text-[var(--text-secondary)]"
              >
                输入: {{ record.inputSummary }}
              </p>
              <p
                v-if="record.outputSummary"
                class="mt-1 text-xs text-[var(--text-secondary)]"
              >
                输出: {{ record.outputSummary }}
              </p>
              <p v-if="record.errorMessage" class="mt-1 text-xs text-red-500">
                错误: {{ record.errorMessage }}
              </p>
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
        :discard-text="confirmDialog.discardText"
        :show-cancel="confirmDialog.showCancel"
        :show-discard="confirmDialog.showDiscard"
        @close="handleConfirmDialogClose"
        @confirm="handleConfirmDialogConfirm"
        @discard="handleConfirmDialogDiscard"
      />

      <DirectoryBrowser
        :is-open="showDirectoryBrowser"
        :initial-path="
          directoryBrowserType === 'comics'
            ? systemSettings.comicsPath
            : cacheSettings.cachePath
        "
        @close="closeDirectoryBrowser"
        @select="handleDirectorySelected"
      />
    </div>
  </BasePageView>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useQuery, useQueryClient } from "@tanstack/vue-query";
import {
  onBeforeRouteLeave,
  onBeforeRouteUpdate,
  useRoute,
  useRouter,
} from "vue-router";
import BasePageView from "@/components/layout/BasePageView.vue";
import BaseModal from "@/components/base/BaseModal.vue";
import ConfirmModal from "@/components/common/ConfirmModal.vue";
import DirectoryBrowser from "@/components/DirectoryBrowser.vue";
import GlassButton from "@/components/base/GlassButton.vue";
import GlassCard from "@/components/base/GlassCard.vue";
import GlassInput from "@/components/base/GlassInput.vue";
import SettingsNav, {
  type SettingsNavItem,
} from "@/components/settings/SettingsNav.vue";
import AppearanceSettingsSection from "@/components/settings/AppearanceSettingsSection.vue";
import SystemSettingsSection from "@/components/settings/SystemSettingsSection.vue";
import UserManagementSection from "@/components/settings/UserManagementSection.vue";
import PluginManagementSection from "@/components/settings/PluginManagementSection.vue";
import BatchOperationsSection from "@/components/settings/BatchOperationsSection.vue";
import AISettingsSection from "@/components/settings/AISettingsSection.vue";
import EmbeddingSettingsSection from "@/components/settings/EmbeddingSettingsSection.vue";
import OCRSettingsSection from "@/components/settings/OCRSettingsSection.vue";
import RecommendationInsightsSection from "@/components/settings/RecommendationInsightsSection.vue";
import TrashSettingsSection from "@/components/settings/TrashSettingsSection.vue";
import { useTheme } from "@/composables/useTheme";
import { useLibraryStore } from "@/stores/library";
import { useTitleDisplayStore } from "@/stores/titleDisplay";
import {
  batchDeleteArchives,
  batchDeleteCategoryArchives,
  batchDeleteTagArchives,
  backfillAITagging,
  backfillAITagLocalizations,
  backfillAITitleTranslations,
  clearCache as apiClearCache,
  controlAITaskQueue,
  configureCache,
  configurePlugin as configurePluginApi,
  createUser,
  deleteUser,
  forceContinueAIModel,
  getAISettings,
  getAIStatus,
  getPendingAITagSuggestions,
  getCacheStatus,
  getCategories,
  getCategoryDeletePreview,
  getPlugin,
  getPluginConfigSchema,
  getPluginExecutions,
  getPlugins,
  getScanSettings,
  getSettings,
  getTags,
  getUsers,
  installPlugin,
  pruneCategories,
  pruneTags,
  previewAITitleTranslation,
  reviewAITagSuggestion,
  testAIConnection,
  togglePlugin,
  triggerScan,
  updateAISettings,
  updateSettings,
  undoAITaggingRun,
} from "@/utils/api";
import { getApiErrorMessage } from "@/utils/error";
import { tagDisplayText } from "@/utils/tagDisplay";
import type { CacheClearScope } from "@/utils/api";
import type {
  AISettings,
  AITaskExecutionSettings,
  AITitleTranslationPreviewResponse,
  AITagSuggestionReviewAction,
  Plugin,
  PluginDetail,
  PluginExecutionRecord,
  ReviewAITagSuggestionRequest,
  ScanSettings,
  SystemSettings,
  User,
} from "@/types/api";
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
const titleDisplayStore = useTitleDisplayStore();

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
    id: "ai-overview",
    name: "智能工作台",
    description: "查看处理能力、队列和异常",
    group: "智能处理",
  },
  {
    id: "ai-models",
    name: "AI 模型",
    description: "对话模型、向量模型和 OCR",
    group: "高级设置",
  },
  {
    id: "ai-tasks",
    name: "AI 任务",
    description: "配置本地化、内容理解、标签和推荐",
    group: "高级设置",
  },
  {
    id: "ai-review",
    name: "审核与任务",
    description: "审核标签建议并处理队列任务",
    group: "智能处理",
  },
  {
    id: "ai-runtime",
    name: "高级运行",
    description: "并发、超时和失败重试策略",
    group: "高级设置",
  },
  {
    id: "trash",
    name: "回收站",
    description: "查看并恢复最近删除的漫画",
    group: "维护与风险",
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
  {
    id: "recommendations",
    name: "推荐洞察",
    description: "随机精选的阅读表现与偏好规则",
    group: "个人偏好",
  },
  {
    id: "trash",
    name: "回收站",
    description: "查看并恢复最近删除的漫画",
    group: "维护与风险",
  },
];

const activeTab = ref("appearance");
const isAdminSettingsRoute = computed(() => route.name === "admin-settings");
const isUserSettingsRoute = computed(() => route.name === "settings");
const AI_PROCESSING_TABS = [
  "ai-overview",
  "ai-models",
  "ai-tasks",
  "ai-review",
  "ai-runtime",
] as const;
const isAIProcessingTab = computed(() =>
  AI_PROCESSING_TABS.includes(
    activeTab.value as (typeof AI_PROCESSING_TABS)[number],
  ),
);
const aiSectionForTab = computed(() => {
  if (activeTab.value === "ai-models") return "models" as const;
  if (activeTab.value === "ai-tasks") return "tasks" as const;
  if (activeTab.value === "ai-review") return "review" as const;
  if (activeTab.value === "ai-runtime") return "runtime" as const;
  return "overview" as const;
});

const pageTitle = computed(() =>
  isAdminSettingsRoute.value ? "管理设置" : "个人设置",
);
const pageSubtitle = computed(() =>
  isAdminSettingsRoute.value
    ? "按分区管理系统配置，危险操作集中在批量维护。"
    : "调整显示偏好，并查看随机精选如何贴近你的阅读选择。",
);

const tabs = computed(() =>
  isAdminSettingsRoute.value ? ADMIN_TABS : USER_TABS,
);

const resolveTabFromQuery = (queryTab: unknown, isAdmin: boolean): string => {
  const rawTab = Array.isArray(queryTab) ? queryTab[0] : queryTab;
  const candidate = typeof rawTab === "string" ? rawTab : "";
  const allowed = (isAdmin ? ADMIN_TABS : USER_TABS).map((item) => item.id);
  const fallback = isAdmin ? "system" : "appearance";
  return allowed.includes(candidate) ? candidate : fallback;
};

const saveCurrentSettings = async (): Promise<boolean> => {
  if (activeTab.value === "system") return saveSystemSettings();
  if (activeTab.value === "ai-models") {
    return saveAIModelSettings();
  }
  if (
    isAIProcessingTab.value &&
    aiSectionForTab.value !== "overview" &&
    aiSectionForTab.value !== "review"
  ) {
    return saveAISettings();
  }
  return true;
};

const confirmUnsavedSettings = async (): Promise<boolean> => {
  const dirty =
    activeTab.value === "system"
      ? isSystemDirty.value
      : activeTab.value === "ai-models"
        ? isAIModelsDirty.value
        : isAIProcessingTab.value &&
          aiSectionForTab.value !== "overview" &&
          aiSectionForTab.value !== "review" &&
          isAIDirty.value;
  if (!dirty) return true;

  const shouldSave = await askForConfirmation({
    title: "保存更改？",
    message: "当前栏目有未保存的更改。保存后再继续，或留在当前页面继续编辑。",
    confirmText: "保存并继续",
    cancelText: "继续编辑",
    showDiscard: true,
    onDiscard: () => {
      if (activeTab.value === "system") discardSystemChanges();
      if (activeTab.value === "ai-models") {
        discardCurrentSettings();
      }
    },
    type: "warning",
  });
  const action = confirmDialogAction;
  confirmDialogAction = "cancel";
  if (action === "discard") return true;
  return shouldSave ? saveCurrentSettings() : false;
};

const setActiveTab = async (tabId: string) => {
  const nextTab = resolveTabFromQuery(tabId, isAdminSettingsRoute.value);
  if (nextTab === activeTab.value || !(await confirmUnsavedSettings())) return;
  activeTab.value = nextTab;
};

watch(
  () => [route.name, route.query.tab],
  ([routeName, queryTab]) => {
    activeTab.value = resolveTabFromQuery(
      queryTab,
      routeName === "admin-settings",
    );
  },
  { immediate: true },
);

watch(
  () => activeTab.value,
  (tab) => {
    const currentTabQuery = Array.isArray(route.query.tab)
      ? route.query.tab[0]
      : route.query.tab;

    if (isAdminSettingsRoute.value) {
      if (currentTabQuery !== tab) {
        void router.replace({
          name: "admin-settings",
          query: { ...route.query, tab },
        });
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
  settingsVersion: 5,
  connection: {
    provider: "openaiCompatible",
    baseUrl: "https://api.openai.com/v1",
    model: "gpt-4o-mini",
    streamResponse: false,
    firstTokenTimeoutSeconds: 30,
    requestIntervalSeconds: 0,
    ollamaUseGpu: false,
    ollamaMaxNumCtx: 16384,
    contextWindowTokens: 16384,
    ollamaThinking: true,
    ollamaRepeatPenalty: 1.15,
    ollamaRepeatLastN: 256,
    visionCapable: true,
    authMode: "bearer",
    apiKeyConfigured: false,
  },
  profiles: [
    {
      id: "default",
      name: "Default",
      enabled: true,
      connection: {
        provider: "openaiCompatible",
        baseUrl: "https://api.openai.com/v1",
        model: "gpt-4o-mini",
        streamResponse: false,
        firstTokenTimeoutSeconds: 30,
        requestIntervalSeconds: 0,
        ollamaUseGpu: false,
        ollamaMaxNumCtx: 16384,
        contextWindowTokens: 16384,
        ollamaThinking: true,
        ollamaRepeatPenalty: 1.15,
        ollamaRepeatLastN: 256,
        visionCapable: true,
        authMode: "bearer",
        apiKeyConfigured: false,
      },
    },
  ],
  activeProfileId: "default",
  execution: {
    lanes: {
      llm: 2,
      ocr: 1,
      plugin: 2,
      orchestration: 2,
    },
    timeoutSeconds: 180,
    maxRetries: 3,
    maxImagesPerTask: 20,
    imageTokenBudget: 1800,
    outputTokenLimit: 2048,
    thinkingOutputTokenLimit: 8192,
    promptSafetyMargin: 1024,
    adaptiveContextRetries: 2,
    ocrMaxPages: 12,
    ocrCharsPerPage: 600,
  },
  features: {
    titleTranslation: {
      enabled: false,
      targetLanguage: "zh-CN",
      skipIfTargetLanguage: true,
      retranslateOnTitleChange: true,
      displayTranslatedTitle: false,
      execution: {
        profileId: "auto",
        thinkingMode: "inherit",
        outputTokenLimit: null,
        thinkingOutputTokenLimit: null,
        thinkingContextWindowTokens: 32768,
        temperature: 0.1,
        structuredOutputMode: "promptOnly",
        maxImagesPerRequest: null,
        timeoutSeconds: null,
        firstTokenTimeoutSeconds: null,
        additionalInstructions: "",
      },
    },
    tagLocalization: {
      enabled: true,
      execution: {
        profileId: "auto",
        thinkingMode: "inherit",
        outputTokenLimit: null,
        thinkingOutputTokenLimit: null,
        thinkingContextWindowTokens: 32768,
        temperature: 0,
        structuredOutputMode: "jsonObject",
        maxImagesPerRequest: null,
        timeoutSeconds: null,
        firstTokenTimeoutSeconds: null,
        additionalInstructions: "",
      },
    },
    contentUnderstanding: {
      execution: {
        profileId: "auto",
        thinkingMode: "inherit",
        outputTokenLimit: null,
        thinkingOutputTokenLimit: 8192,
        thinkingContextWindowTokens: 32768,
        temperature: 0,
        structuredOutputMode: "jsonObject",
        maxImagesPerRequest: 4,
        timeoutSeconds: null,
        firstTokenTimeoutSeconds: null,
        additionalInstructions: "",
      },
    },
    autoTagging: {
      enabled: false,
      mode: "autoApplyReliable",
      autoProcessNewArchives: true,
      execution: {
        profileId: "auto",
        thinkingMode: "inherit",
        outputTokenLimit: null,
        thinkingOutputTokenLimit: null,
        thinkingContextWindowTokens: 32768,
        temperature: 0,
        structuredOutputMode: "jsonObject",
        maxImagesPerRequest: 4,
        timeoutSeconds: null,
        firstTokenTimeoutSeconds: null,
        additionalInstructions: "",
      },
    },
    recommendations: {
      multiUserExperimentEnabled: false,
      analysisRefreshAfterDays: 180,
    },
  },
});

const updateAIExecutionLane = (
  lane: "llm" | "ocr" | "plugin" | "orchestration",
  limit: number,
) => {
  aiSettings.value.execution.lanes[lane] = limit;
};

const systemLoading = ref(false);
const systemSaveError = ref<string | null>(null);
const scanLoading = ref(false);
const scanResult = ref<{ success: boolean; message: string } | null>(null);
const aiLoading = ref(false);
const aiModelSaving = ref(false);
const aiSaveError = ref<string | null>(null);
const testingAIConnection = ref(false);
const previewingTitleTranslation = ref(false);
const titleTranslationPreview = ref<AITitleTranslationPreviewResponse | null>(
  null,
);
const backfillingTitleTranslations = ref(false);
const repairingTitleTranslations = ref(false);
const retranslatingTitleTranslations = ref(false);
const backfillingAITagging = ref(false);
const backfillingAITagLocalizations = ref(false);
const controllingAITaskQueue = ref<string | null>(null);
const controllingAIModel = ref<string | null>(null);
const reviewingAITagSuggestionId = ref<string | null>(null);
const undoingAITaggingRunId = ref<string | null>(null);
const recentTaggingRunIds = ref<string[]>([]);
const tagSuggestionPage = ref(1);
const cacheStatus = ref<CacheStatusResponse | null>(null);
const clearingCacheScope = ref<CacheClearScope | null>(null);
const isClearingCache = computed(() => clearingCacheScope.value !== null);
const ocrDirty = ref(false);
const ocrSettingsRef = ref<InstanceType<typeof OCRSettingsSection> | null>(
  null,
);
const embeddingDirty = ref(false);
const embeddingSettingsRef = ref<InstanceType<
  typeof EmbeddingSettingsSection
> | null>(null);

const showCreateUserModal = ref(false);
const createUserForm = ref({ username: "", email: "", password: "" });
const createUserLoading = ref(false);

const showInstallPluginModal = ref(false);
const pluginFileInput = ref<HTMLInputElement>();
const installPluginLoading = ref(false);

type JsonObject = Record<string, unknown>;

interface PluginDetailSnapshot {
  permissions?: unknown;
  configurable?: boolean;
  loaded?: boolean;
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
  showDiscard?: boolean;
  discardText?: string;
  onDiscard?: () => void;
}

const confirmDialog = ref({
  show: false,
  title: "确认操作",
  message: "",
  type: "default" as ConfirmDialogType,
  confirmText: "确认",
  cancelText: "取消",
  discardText: "放弃更改",
  showCancel: true,
  showDiscard: false,
});

let confirmDialogResolver: ((result: boolean) => void) | null = null;
let confirmDialogDiscardAction: (() => void) | null = null;
let confirmDialogAction: "confirm" | "cancel" | "discard" = "cancel";

const askForConfirmation = (
  options: ConfirmDialogOptions,
): Promise<boolean> => {
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
    discardText: options.discardText ?? "放弃更改",
    showDiscard: options.showDiscard ?? false,
  };
  confirmDialogDiscardAction = options.onDiscard ?? null;
  confirmDialogAction = "cancel";

  return new Promise((resolve) => {
    confirmDialogResolver = resolve;
  });
};

const resolveConfirmDialog = (result: boolean) => {
  confirmDialog.value.show = false;
  confirmDialogDiscardAction = null;
  if (confirmDialogResolver) {
    confirmDialogResolver(result);
    confirmDialogResolver = null;
  }
};

const handleConfirmDialogClose = () => {
  confirmDialogAction = "cancel";
  resolveConfirmDialog(false);
};

const handleConfirmDialogConfirm = () => {
  confirmDialogAction = "confirm";
  resolveConfirmDialog(true);
};

const handleConfirmDialogDiscard = () => {
  confirmDialogAction = "discard";
  confirmDialogDiscardAction?.();
  confirmDialogDiscardAction = null;
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

const runSettingsAction = async <T,>(
  action: () => Promise<T>,
  options: SettingsActionOptions,
): Promise<T | null> => {
  try {
    const result = await action();
    if (options.successMessage) {
      await showInfoDialog(
        options.successTitle ?? "操作成功",
        options.successMessage,
      );
    }
    return result;
  } catch (error) {
    console.error(options.logLabel, error);
    await showInfoDialog(
      "操作失败",
      getApiErrorMessage(error, options.fallbackErrorMessage),
    );
    return null;
  }
};

const usersQuery = useQuery({
  queryKey: ["users"],
  queryFn: getUsers,
  enabled: computed(
    () => isAdminSettingsRoute.value && activeTab.value === "users",
  ),
});

const pluginsQuery = useQuery({
  queryKey: ["plugins"],
  queryFn: getPlugins,
  enabled: computed(
    () => isAdminSettingsRoute.value && activeTab.value === "plugins",
  ),
});

const aiStatusQuery = useQuery({
  queryKey: ["ai-status"],
  queryFn: getAIStatus,
  enabled: computed(
    () => isAdminSettingsRoute.value && isAIProcessingTab.value,
  ),
  refetchInterval: 5000,
});

const TAG_SUGGESTION_PAGE_SIZE = 20;

const aiTagSuggestionsQuery = useQuery({
  queryKey: ["ai-tag-suggestions", tagSuggestionPage],
  queryFn: () =>
    getPendingAITagSuggestions({
      limit: TAG_SUGGESTION_PAGE_SIZE,
      page: tagSuggestionPage.value,
      includeAutoApplied: true,
    }),
  enabled: computed(
    () => isAdminSettingsRoute.value && activeTab.value === "ai-review",
  ),
  refetchInterval: 5000,
});

const categoriesQuery = useQuery({
  queryKey: ["categories"],
  queryFn: getCategories,
  enabled: computed(
    () => isAdminSettingsRoute.value && activeTab.value === "batch",
  ),
});

const tagsQuery = useQuery({
  queryKey: ["tags"],
  queryFn: getTags,
  enabled: computed(
    () => isAdminSettingsRoute.value && activeTab.value === "batch",
  ),
});

const users = computed(() => usersQuery.data.value ?? []);
const usersLoading = computed(() => usersQuery.isLoading.value);
const plugins = computed(() => pluginsQuery.data.value ?? []);
const pluginsLoading = computed(() => pluginsQuery.isLoading.value);
const aiStatus = computed(() => aiStatusQuery.data.value);
const tagSuggestions = computed(
  () => aiTagSuggestionsQuery.data.value?.items ?? [],
);
const tagSuggestionsTotal = computed(
  () => aiTagSuggestionsQuery.data.value?.total ?? 0,
);
const tagSuggestionPageCount = computed(() =>
  Math.max(1, Math.ceil(tagSuggestionsTotal.value / TAG_SUGGESTION_PAGE_SIZE)),
);
const loadingAITagSuggestions = computed(
  () => aiTagSuggestionsQuery.isFetching.value,
);
watch(tagSuggestionPageCount, (pageCount) => {
  if (aiTagSuggestionsQuery.data.value && tagSuggestionPage.value > pageCount) {
    tagSuggestionPage.value = pageCount;
  }
});
const categories = computed(() => categoriesQuery.data.value ?? []);
const tags = computed(() => tagsQuery.data.value ?? []);

const asObject = (value: unknown): JsonObject | null => {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return null;
  }
  return value as JsonObject;
};

const getStringFromObject = (
  source: JsonObject | null,
  key: string,
): string | undefined => {
  if (!source) return undefined;
  const value = source[key];
  if (typeof value === "string" && value.trim().length > 0) {
    return value;
  }
  return undefined;
};

const cloneValue = <T,>(value: T): T => {
  if (value === null || typeof value !== "object") {
    return value;
  }
  return JSON.parse(JSON.stringify(value)) as T;
};

const defaultTaskExecution = (
  temperature = 0,
  maxImagesPerRequest: number | null = null,
  structuredOutputMode: AITaskExecutionSettings["structuredOutputMode"] = "jsonObject",
  thinkingOutputTokenLimit: number | null = null,
): AITaskExecutionSettings => ({
  profileId: "auto",
  thinkingMode: "inherit",
  outputTokenLimit: null,
  thinkingOutputTokenLimit,
  thinkingContextWindowTokens: 32768,
  temperature,
  structuredOutputMode,
  maxImagesPerRequest,
  timeoutSeconds: null,
  firstTokenTimeoutSeconds: null,
  additionalInstructions: "",
});

const normalizeTaskExecution = (
  value: Partial<AITaskExecutionSettings> | undefined,
  fallback: AITaskExecutionSettings = defaultTaskExecution(),
  maxImagesPerRequestLimit = 64,
  enabledProfileIds?: ReadonlySet<string>,
  supportsJsonSchema = false,
): AITaskExecutionSettings => {
  const outputTokenLimit = value?.outputTokenLimit;
  const thinkingOutputTokenLimit = value?.thinkingOutputTokenLimit;
  const thinkingContextWindowTokens = value?.thinkingContextWindowTokens;
  const temperature = value?.temperature;
  const maxImagesPerRequest = value?.maxImagesPerRequest;
  const timeoutSeconds = value?.timeoutSeconds;
  const firstTokenTimeoutSeconds = value?.firstTokenTimeoutSeconds;
  return {
    profileId:
      typeof value?.profileId === "string" &&
      (value.profileId === "auto" || enabledProfileIds?.has(value.profileId))
        ? value.profileId
        : fallback.profileId,
    thinkingMode:
      value?.thinkingMode === "inherit" ||
      value?.thinkingMode === "disabled" ||
      value?.thinkingMode === "enabled"
        ? value.thinkingMode
        : fallback.thinkingMode,
    outputTokenLimit:
      Number.isFinite(outputTokenLimit) && (outputTokenLimit as number) >= 32
        ? (outputTokenLimit as number)
        : null,
    thinkingOutputTokenLimit:
      Number.isFinite(thinkingOutputTokenLimit) &&
      (thinkingOutputTokenLimit as number) >= 32
        ? (thinkingOutputTokenLimit as number)
        : null,
    thinkingContextWindowTokens:
      thinkingContextWindowTokens === null
        ? null
        : Number.isFinite(thinkingContextWindowTokens) &&
            (thinkingContextWindowTokens as number) >= 16384
          ? (thinkingContextWindowTokens as number)
          : fallback.thinkingContextWindowTokens,
    temperature:
      Number.isFinite(temperature) &&
      (temperature as number) >= 0 &&
      (temperature as number) <= 2
        ? (temperature as number)
        : fallback.temperature,
    structuredOutputMode:
      value?.structuredOutputMode === "jsonObject" ||
      value?.structuredOutputMode === "promptOnly" ||
      (supportsJsonSchema && value?.structuredOutputMode === "jsonSchema")
        ? value.structuredOutputMode
        : fallback.structuredOutputMode,
    maxImagesPerRequest:
      maxImagesPerRequest === null
        ? null
        : Number.isFinite(maxImagesPerRequest) &&
            (maxImagesPerRequest as number) >= 1 &&
            (maxImagesPerRequest as number) <= maxImagesPerRequestLimit
          ? (maxImagesPerRequest as number)
          : Number.isFinite(maxImagesPerRequest) &&
              (maxImagesPerRequest as number) > maxImagesPerRequestLimit
            ? maxImagesPerRequestLimit
            : fallback.maxImagesPerRequest,
    timeoutSeconds:
      Number.isFinite(timeoutSeconds) && (timeoutSeconds as number) >= 5
        ? (timeoutSeconds as number)
        : null,
    firstTokenTimeoutSeconds:
      Number.isFinite(firstTokenTimeoutSeconds) &&
      (firstTokenTimeoutSeconds as number) >= 1
        ? (firstTokenTimeoutSeconds as number)
        : null,
    additionalInstructions:
      typeof value?.additionalInstructions === "string"
        ? value.additionalInstructions.slice(0, 2000)
        : "",
  };
};

const normalizeLoadedAISettings = (settings: AISettings): AISettings => {
  const enabledProfileIds = new Set(
    settings.profiles
      .filter((profile) => profile.enabled)
      .map((profile) => profile.id),
  );
  const autoTagging = settings.features.autoTagging;
  const tagLocalization = settings.features.tagLocalization ?? {
    enabled: true,
    execution: defaultTaskExecution(),
  };
  const contentUnderstanding = settings.features.contentUnderstanding ?? {
    execution: defaultTaskExecution(0, 4, "jsonObject", 8192),
  };
  const lanes = settings.execution.lanes ?? {
    llm: 2,
    ocr: 1,
    plugin: 2,
    orchestration: 2,
  };
  const defaultVisionImages = Math.min(
    4,
    settings.execution.maxImagesPerTask ?? 20,
  );
  const recommendations = settings.features.recommendations ?? {
    multiUserExperimentEnabled: false,
    analysisRefreshAfterDays: 180,
  };
  const titleExecution = normalizeTaskExecution(
    settings.features.titleTranslation.execution,
    defaultTaskExecution(0.1, null, "promptOnly"),
    64,
    enabledProfileIds,
    true,
  );
  return {
    ...settings,
    connection: {
      ...settings.connection,
      contextWindowTokens:
        Number.isFinite(settings.connection.contextWindowTokens) &&
        settings.connection.contextWindowTokens >= 1024
          ? settings.connection.contextWindowTokens
          : settings.connection.ollamaMaxNumCtx || 16384,
    },
    profiles: settings.profiles.map((profile) => ({
      ...profile,
      connection: {
        ...profile.connection,
        contextWindowTokens:
          Number.isFinite(profile.connection.contextWindowTokens) &&
          profile.connection.contextWindowTokens >= 1024
            ? profile.connection.contextWindowTokens
            : profile.connection.ollamaMaxNumCtx || 16384,
      },
    })),
    execution: {
      ...settings.execution,
      maxImagesPerTask: settings.execution.maxImagesPerTask ?? 20,
      imageTokenBudget: settings.execution.imageTokenBudget ?? 1800,
      outputTokenLimit: settings.execution.outputTokenLimit ?? 2048,
      thinkingOutputTokenLimit:
        settings.execution.thinkingOutputTokenLimit ?? 8192,
      promptSafetyMargin: settings.execution.promptSafetyMargin ?? 1024,
      adaptiveContextRetries: settings.execution.adaptiveContextRetries ?? 2,
      ocrMaxPages: settings.execution.ocrMaxPages ?? 12,
      ocrCharsPerPage: settings.execution.ocrCharsPerPage ?? 600,
      lanes: {
        llm: Number.isFinite(lanes.llm) && lanes.llm >= 1 ? lanes.llm : 2,
        ocr: Number.isFinite(lanes.ocr) && lanes.ocr >= 1 ? lanes.ocr : 1,
        plugin:
          Number.isFinite(lanes.plugin) && lanes.plugin >= 1 ? lanes.plugin : 2,
        orchestration:
          Number.isFinite(lanes.orchestration) && lanes.orchestration >= 1
            ? lanes.orchestration
            : 2,
      },
    },
    features: {
      ...settings.features,
      titleTranslation: {
        enabled: settings.features.titleTranslation.enabled,
        targetLanguage: settings.features.titleTranslation.targetLanguage,
        skipIfTargetLanguage:
          settings.features.titleTranslation.skipIfTargetLanguage,
        retranslateOnTitleChange:
          settings.features.titleTranslation.retranslateOnTitleChange,
        displayTranslatedTitle:
          settings.features.titleTranslation.displayTranslatedTitle,
        execution: titleExecution,
      },
      tagLocalization: {
        enabled: tagLocalization.enabled !== false,
        execution: {
          ...normalizeTaskExecution(
            tagLocalization.execution,
            defaultTaskExecution(),
            64,
            enabledProfileIds,
          ),
          additionalInstructions: "",
        },
      },
      contentUnderstanding: {
        execution: normalizeTaskExecution(
          contentUnderstanding.execution,
          defaultTaskExecution(0, defaultVisionImages, "jsonObject", 8192),
          settings.execution.maxImagesPerTask ?? 20,
          enabledProfileIds,
        ),
      },
      autoTagging: {
        ...autoTagging,
        mode:
          autoTagging.mode === "suggestions"
            ? "suggestions"
            : "autoApplyReliable",
        autoProcessNewArchives: autoTagging.autoProcessNewArchives !== false,
        execution: normalizeTaskExecution(
          autoTagging.execution,
          defaultTaskExecution(0, defaultVisionImages),
          settings.execution.maxImagesPerTask ?? 20,
          enabledProfileIds,
        ),
      },
      recommendations: {
        multiUserExperimentEnabled:
          recommendations.multiUserExperimentEnabled === true,
        analysisRefreshAfterDays:
          Number.isFinite(recommendations.analysisRefreshAfterDays) &&
          recommendations.analysisRefreshAfterDays >= 30 &&
          recommendations.analysisRefreshAfterDays <= 730
            ? recommendations.analysisRefreshAfterDays
            : 180,
      },
    },
  };
};

interface SystemSettingsSnapshot {
  systemSettings: SystemSettings;
  cacheSettings: CacheSettingsForm;
  scanSettings: ScanSettings;
}

const savedSystemSettings = ref<SystemSettingsSnapshot | null>(null);
const savedAISettings = ref<AISettings | null>(null);
const systemSavedMessage = ref<string | null>(null);
const aiSavedMessage = ref<string | null>(null);

const isSystemDirty = computed(() => {
  if (!savedSystemSettings.value) return false;
  return (
    JSON.stringify({
      systemSettings: systemSettings.value,
      cacheSettings: cacheSettings.value,
      scanSettings: scanSettings.value,
    }) !== JSON.stringify(savedSystemSettings.value)
  );
});

const isAIDirty = computed(() => {
  if (!savedAISettings.value) return false;
  return (
    JSON.stringify(aiSettings.value) !== JSON.stringify(savedAISettings.value)
  );
});

const isAIModelsDirty = computed(
  () => isAIDirty.value || ocrDirty.value || embeddingDirty.value,
);

const isAIModelsLoading = computed(
  () => aiLoading.value || aiModelSaving.value,
);

const markSystemSettingsSaved = (message = "配置已保存") => {
  savedSystemSettings.value = cloneValue({
    systemSettings: systemSettings.value,
    cacheSettings: cacheSettings.value,
    scanSettings: scanSettings.value,
  });
  systemSavedMessage.value = message;
  systemSaveError.value = null;
};

const markAISettingsSaved = (message = "配置已保存") => {
  savedAISettings.value = cloneValue(aiSettings.value);
  aiSavedMessage.value = message;
  aiSaveError.value = null;
};

const discardSystemChanges = () => {
  if (!savedSystemSettings.value) return;
  const saved = cloneValue(savedSystemSettings.value);
  systemSettings.value = saved.systemSettings;
  cacheSettings.value = saved.cacheSettings;
  scanSettings.value = saved.scanSettings;
  systemSavedMessage.value = "已放弃未保存的更改";
  systemSaveError.value = null;
};

const discardAIChanges = () => {
  if (!savedAISettings.value) return;
  aiSettings.value = cloneValue(savedAISettings.value);
  aiSavedMessage.value = "已放弃未保存的更改";
  aiSaveError.value = null;
};

const discardCurrentSettings = () => {
  if (activeTab.value !== "ai-models") {
    discardAIChanges();
    return;
  }
  discardAIChanges();
  ocrSettingsRef.value?.discard?.();
  embeddingSettingsRef.value?.discard?.();
  aiSavedMessage.value = "已放弃未保存的模型设置";
};

const dirtyTabs = computed(() => {
  const dirty = new Set<string>();
  if (isSystemDirty.value) dirty.add("system");
  if (activeTab.value === "ai-models" && isAIModelsDirty.value) {
    dirty.add("ai-models");
  } else if (isAIDirty.value && isAIProcessingTab.value) {
    dirty.add(activeTab.value);
  }
  return Array.from(dirty);
});

const mergePluginDetail = (pluginId: string, patch: PluginDetailSnapshot) => {
  const previous = pluginDetails.value[pluginId] ?? {};
  pluginDetails.value = {
    ...pluginDetails.value,
    [pluginId]: { ...previous, ...patch },
  };
};

const updatePluginDetail = (
  pluginId: string,
  detail: PluginDetail,
): PluginDetail => {
  const schema = extractConfigSchema(detail.manifest);
  mergePluginDetail(pluginId, {
    permissions: detail.permissions,
    configurable: Boolean(
      schema?.properties && Object.keys(schema.properties).length > 0,
    ),
    loaded: true,
  });
  return detail;
};

const fetchPluginDetail = async (
  pluginId: string,
): Promise<PluginDetail | null> => {
  if (pluginDetailLoadingIds.has(pluginId)) {
    return null;
  }

  pluginDetailLoadingIds.add(pluginId);
  try {
    return updatePluginDetail(pluginId, await getPlugin(pluginId));
  } catch (error) {
    console.error("获取插件详情失败:", error);
    return null;
  } finally {
    pluginDetailLoadingIds.delete(pluginId);
  }
};

const hydratePluginDetails = async (pluginList: Plugin[]) => {
  if (pluginList.length === 0) return;

  await Promise.all(
    pluginList.map(async (plugin) => {
      const cached = pluginDetails.value[plugin.plugin_id];
      if (cached?.loaded) return;
      await fetchPluginDetail(plugin.plugin_id);
    }),
  );
};

watch(
  plugins,
  (pluginList) => {
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
      void hydratePluginDetails(plugins.value);
    }
  },
);

const toStringArray = (value: unknown): string[] => {
  if (!Array.isArray(value)) return [];
  return value.filter(
    (item): item is string => typeof item === "string" && item.length > 0,
  );
};

const formatPermissionSummary = (permissions: unknown): string[] => {
  if (!permissions || typeof permissions !== "object") {
    return ["未声明权限"];
  }

  const permissionData = permissions as Record<string, unknown>;
  const summary: string[] = [];

  summary.push(permissionData.network === true ? "网络(开放)" : "无网络");

  const fsRead = toStringArray(permissionData.filesystem_read);
  if (fsRead.length > 0) summary.push(`读文件(${fsRead.length})`);

  const fsWrite = toStringArray(permissionData.filesystem_write);
  if (fsWrite.length > 0) summary.push(`写文件(${fsWrite.length})`);

  const dbRead = permissionData.database_read;
  if (typeof dbRead === "boolean") {
    summary.push(dbRead ? "数据库读" : "无数据库读");
  }

  const dbWrite = toStringArray(permissionData.database_write);
  if (dbWrite.length > 0) summary.push(`数据库写(${dbWrite.length})`);

  return summary;
};

const normalizeSchemaField = (value: unknown): PluginSchemaField => {
  const field = asObject(value);
  if (!field) return {};

  return {
    type: getStringFromObject(field, "type"),
    title: getStringFromObject(field, "title"),
    description: getStringFromObject(field, "description"),
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
    ? rawSchema.required.filter(
        (item): item is string => typeof item === "string",
      )
    : [];

  return {
    type: getStringFromObject(rawSchema, "type"),
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

  return normalizePluginSchema(raw.config_schema);
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
  return `${target.name} 配置`;
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
  return formatPermissionSummary(
    pluginDetails.value[target.plugin_id]?.permissions,
  );
});

const getSchemaEnumOptions = (enumValues: unknown[] | undefined): string[] => {
  if (!enumValues) return [];
  return enumValues.map((option) => String(option));
};

const toNumberInputValue = (value: unknown): string => {
  return typeof value === "number" && Number.isFinite(value)
    ? String(value)
    : "";
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

const setSchemaFieldFromInput = (
  key: string,
  schema: PluginSchemaField,
  rawValue: string,
) => {
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

const onSchemaInputEvent = (
  key: string,
  schema: PluginSchemaField,
  event: Event,
) => {
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
  const objectFields = pluginConfigSchemaFields.value.filter(
    (field) => field.schema.type === "object",
  );
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
    const isEmptyString =
      typeof value === "string" && value.trim().length === 0;
    const isEmptyArray = Array.isArray(value) && value.length === 0;
    if (
      value === undefined ||
      value === null ||
      isEmptyString ||
      isEmptyArray
    ) {
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
  const pluginId = plugin.plugin_id;

  showPluginConfigModal.value = true;
  pluginConfigTarget.value = plugin;
  pluginConfigLoading.value = true;
  pluginConfigFormError.value = "";
  pluginConfigFieldErrors.value = {};

  try {
    const detail = await fetchPluginDetail(pluginId);
    if (!detail) {
      throw new Error("获取插件详情失败");
    }

    const schemaResponse = await getPluginConfigSchema(pluginId);
    const schema = normalizePluginSchema(schemaResponse.config_schema) ?? {
      type: "object",
      properties: {},
      required: [],
    };

    pluginConfigSchema.value = schema;

    const currentConfig = asObject(detail?.config) ?? {};

    const initializedConfig = initializePluginConfigData(schema, currentConfig);
    pluginConfigFormData.value = initializedConfig;
    pluginConfigFieldDrafts.value = createConfigFieldDrafts(
      schema,
      initializedConfig,
    );
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

  const pluginId = target.plugin_id;

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

const toPluginExecutionRecordView = (
  record: PluginExecutionRecord,
): PluginExecutionRecordView => {
  return {
    executionId: record.execution_id,
    pluginId: record.plugin_id,
    archiveId: record.archive_id ?? null,
    executionType: record.execution_type,
    status: record.status,
    inputSummary: record.input_summary ?? null,
    outputSummary: record.output_summary ?? null,
    errorMessage: record.error_message ?? null,
    durationMs: record.duration_ms ?? null,
    startedAt: record.started_at,
    completedAt: record.completed_at ?? null,
  };
};

const pluginExecutionsModalTitle = computed(() => {
  const target = pluginExecutionTarget.value as Plugin | null;
  if (!target) return "执行记录";
  return `${target.name} 执行记录`;
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

  const pluginId = target.plugin_id;

  pluginExecutionLoading.value = true;
  try {
    const response = await getPluginExecutions(pluginId, {
      limit: 50,
      ...(pluginExecutionStatusFilter.value !== "all"
        ? { status: pluginExecutionStatusFilter.value }
        : {}),
    });
    pluginExecutionRecords.value = response.items.map(
      toPluginExecutionRecordView,
    );
    pluginExecutionTotal.value = response.total;
  } catch (error) {
    console.error("加载插件执行记录失败:", error);
    await showInfoDialog(
      "操作失败",
      getApiErrorMessage(error, "加载插件执行记录失败"),
    );
  } finally {
    pluginExecutionLoading.value = false;
  }
};

const openPluginExecutions = async (plugin: Plugin) => {
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

const saveSystemSettings = async (): Promise<boolean> => {
  systemLoading.value = true;
  systemSaveError.value = null;
  let saved = false;
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
                  max_cached_archives:
                    cacheSettings.value.customConfig.maxCachedArchives,
                  cache_ttl_hours:
                    cacheSettings.value.customConfig.cacheTtlHours,
                  preload_prev_pages:
                    cacheSettings.value.customConfig.preloadPrevPages,
                  preload_next_pages:
                    cacheSettings.value.customConfig.preloadNextPages,
                }
              : undefined,
        });
        saved = true;
      },
      {
        logLabel: "保存系统设置失败:",
        fallbackErrorMessage: "保存失败，请稍后重试",
      },
    );
    if (saved) {
      markSystemSettingsSaved();
    } else {
      systemSaveError.value = "无法保存系统设置，请检查输入后重试。";
    }
    return saved;
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
      await showInfoDialog(
        "操作完成",
        result.message || `${scopeNameMap[scope]}已清理`,
      );
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

const saveAISettings = async (): Promise<boolean> => {
  aiLoading.value = true;
  aiSaveError.value = null;
  let saved = false;
  try {
    await runSettingsAction(
      async () => {
        await updateAISettings(aiSettings.value);
        titleDisplayStore.setEnabled(
          aiSettings.value.features.titleTranslation.displayTranslatedTitle,
        );
        saved = true;
      },
      {
        logLabel: "保存AI设置失败:",
        fallbackErrorMessage: "保存失败，请稍后重试",
      },
    );
    if (saved) {
      markAISettingsSaved();
    } else {
      aiSaveError.value = "无法保存 AI 设置，请检查连接和配置后重试。";
    }
    return saved;
  } finally {
    aiLoading.value = false;
  }
};

const saveAIModelSettings = async (): Promise<boolean> => {
  aiModelSaving.value = true;
  aiSaveError.value = null;
  const results: string[] = [];
  try {
    if (isAIDirty.value) {
      results.push(
        (await saveAISettings()) ? "对话模型已保存" : "对话模型保存失败",
      );
    }

    if (embeddingDirty.value) {
      const saved = (await embeddingSettingsRef.value?.save?.()) ?? false;
      results.push(saved ? "向量模型已保存" : "向量模型保存失败");
    }

    if (ocrDirty.value) {
      const saved = (await ocrSettingsRef.value?.save?.()) ?? false;
      results.push(saved ? "OCR 已保存" : "OCR 保存失败");
    }

    const failures = results.filter((result) => result.endsWith("失败"));
    if (failures.length > 0) {
      aiSaveError.value = `${results.join("；")}。请修正失败区块后重试。`;
      return false;
    }

    aiSavedMessage.value = results.join("；") || "AI 模型设置已保存";
    return true;
  } finally {
    aiModelSaving.value = false;
  }
};

const handleControlAITaskQueue = async (
  jobTypes: readonly string[],
  action: "pause" | "resume" | "forceContinue",
) => {
  const normalizedJobTypes = [...new Set(jobTypes)];
  if (normalizedJobTypes.length === 0) return;
  const controlKey =
    normalizedJobTypes.length === 1
      ? normalizedJobTypes[0]
      : "content_analysis";
  controllingAITaskQueue.value = controlKey;
  try {
    const completed = await runSettingsAction(
      async () => {
        await Promise.all(
          normalizedJobTypes.map((jobType) =>
            controlAITaskQueue(jobType, action),
          ),
        );
        return true;
      },
      {
        logLabel: "更新任务队列状态失败:",
        fallbackErrorMessage: "无法更新任务队列状态",
      },
    );
    if (completed) {
      await queryClient.invalidateQueries({ queryKey: ["ai-status"] });
    }
  } finally {
    controllingAITaskQueue.value = null;
  }
};

const handleForceContinueAIModel = async (profileId: string) => {
  controllingAIModel.value = profileId;
  try {
    const completed = await runSettingsAction(
      async () => {
        await forceContinueAIModel(profileId);
        return true;
      },
      {
        logLabel: "强制继续模型失败:",
        fallbackErrorMessage: "无法强制继续该模型",
      },
    );
    if (completed) {
      await queryClient.invalidateQueries({ queryKey: ["ai-status"] });
    }
  } finally {
    controllingAIModel.value = null;
  }
};

const handleTestAIConnection = async () => {
  testingAIConnection.value = true;
  try {
    const result = await runSettingsAction(
      () => testAIConnection(aiSettings.value),
      {
        logLabel: "测试 AI 连接失败:",
        fallbackErrorMessage: "无法验证 AI 连接",
      },
    );

    if (!result) return;
    await showInfoDialog(
      result.success ? "AI 连接可用" : "AI 连接不可用",
      result.message ||
        (result.success
          ? "该配置已通过连接验证。"
          : "请检查服务地址、模型名称、认证配置和模型能力。"),
    );
  } finally {
    testingAIConnection.value = false;
  }
};

const handlePreviewTitleTranslation = async (title: string) => {
  previewingTitleTranslation.value = true;
  titleTranslationPreview.value = null;
  try {
    titleTranslationPreview.value = await previewAITitleTranslation(
      title,
      aiSettings.value.features.titleTranslation.targetLanguage,
      aiSettings.value,
    );
  } catch (error) {
    titleTranslationPreview.value = {
      success: false,
      message: getApiErrorMessage(error, "标题翻译试运行失败"),
      preview: null,
    };
  } finally {
    previewingTitleTranslation.value = false;
  }
};

const handleBackfillTitleTranslations = async () => {
  const confirmed = await askForConfirmation({
    title: "确认补充标题翻译",
    message:
      "系统会检查书库中缺少目标语言译文的标题，并将符合条件的项目加入后台队列。配置会在确认后保存。",
    type: "warning",
    confirmText: "加入队列",
  });
  if (!confirmed) return;

  if (isAIDirty.value && !(await saveAISettings())) return;

  backfillingTitleTranslations.value = true;
  try {
    const result = await runSettingsAction(backfillAITitleTranslations, {
      logLabel: "批量补翻译失败:",
      fallbackErrorMessage: "无法创建标题翻译任务",
    });

    if (!result) return;
    aiSavedMessage.value = "已加入翻译队列。";
    await queryClient.invalidateQueries({ queryKey: ["ai-status"] });
  } finally {
    backfillingTitleTranslations.value = false;
  }
};

const handleRepairSuspiciousTitleTranslations = async () => {
  const confirmed = await askForConfirmation({
    title: "确认修复标题翻译",
    message:
      "系统会筛选失败或疑似拒答的标题并重新加入队列。配置会在确认后保存。",
    type: "warning",
    confirmText: "加入队列",
  });
  if (!confirmed) return;

  if (isAIDirty.value && !(await saveAISettings())) return;

  repairingTitleTranslations.value = true;
  try {
    const result = await runSettingsAction(
      () => backfillAITitleTranslations(false, true),
      {
        logLabel: "修复失败或疑似拒答标题失败:",
        fallbackErrorMessage: "无法创建标题修复任务",
      },
    );

    if (!result) return;
    aiSavedMessage.value =
      "已开始筛选失败或疑似拒答的标题，并仅重新加入可疑项。";
    await queryClient.invalidateQueries({ queryKey: ["ai-status"] });
  } finally {
    repairingTitleTranslations.value = false;
  }
};

const handleForceRetranslateTitleTranslations = async () => {
  const confirmed = await askForConfirmation({
    title: "确认重新翻译",
    message:
      "现有副标题将重新加入翻译队列，并在新译文完成前暂时隐藏。此操作会产生额外的模型请求。",
    type: "warning",
    confirmText: "重新翻译",
  });
  if (!confirmed) return;

  if (isAIDirty.value && !(await saveAISettings())) return;

  retranslatingTitleTranslations.value = true;
  try {
    const result = await runSettingsAction(
      () => backfillAITitleTranslations(true),
      {
        logLabel: "重新翻译已有标题失败:",
        fallbackErrorMessage: "无法创建重新翻译任务",
      },
    );

    if (!result) return;
    aiSavedMessage.value = "已加入重新翻译队列。";
    await queryClient.invalidateQueries({ queryKey: ["ai-status"] });
  } finally {
    retranslatingTitleTranslations.value = false;
  }
};

const refreshAITagSuggestions = async () => {
  await aiTagSuggestionsQuery.refetch();
};

const changeAITagSuggestionsPage = (page: number) => {
  if (page < 1 || page > tagSuggestionPageCount.value) return;
  tagSuggestionPage.value = page;
};

const handleBackfillAITagging = async () => {
  const confirmed = await askForConfirmation({
    title: "确认批量分析和打标签",
    message:
      "系统会检查整个书库，并将需要重新分析的漫画加入内容分析和标签队列。任务会持续在后台运行，配置会在确认后保存。",
    type: "warning",
    confirmText: "加入队列",
  });
  if (!confirmed) return;

  if (isAIDirty.value && !(await saveAISettings())) return;

  backfillingAITagging.value = true;
  try {
    const result = await runSettingsAction(
      async () => {
        let cursor: string | undefined;
        let queued = 0;
        let skipped = 0;
        let attempted = 0;
        let failed = 0;

        for (;;) {
          const batch = await backfillAITagging({ limit: 100, cursor });
          queued += batch.queued;
          skipped += batch.skipped;
          attempted += batch.attempted;
          failed += batch.failed;
          if (!batch.hasMore) break;
          if (!batch.nextCursor) {
            throw new Error("批量分析未返回下一页游标");
          }
          cursor = batch.nextCursor;
        }

        return { queued, skipped, attempted, failed };
      },
      {
        logLabel: "批量内容分析和自动标签失败:",
        fallbackErrorMessage: "无法创建内容分析与自动标签任务",
      },
    );
    if (!result) return;

    aiSavedMessage.value = `已检查 ${result.attempted} 本漫画，将 ${result.queued} 本加入内容分析与自动标签队列。${result.skipped > 0 ? ` ${result.skipped} 本已有有效分析，未重复加入。` : ""}${result.failed > 0 ? ` ${result.failed} 本加入失败，可稍后再次运行。` : ""}`;
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["ai-status"] }),
      queryClient.invalidateQueries({ queryKey: ["ai-tag-suggestions"] }),
    ]);
  } finally {
    backfillingAITagging.value = false;
  }
};

const handleBackfillAITagLocalizations = async () => {
  const confirmed = await askForConfirmation({
    title: "确认生成标签中文名",
    message:
      "所有尚未翻译的英文规范标签会加入后台翻译队列。已完成或人工维护的中文名称不会被覆盖。",
    type: "warning",
    confirmText: "加入队列",
  });
  if (!confirmed) return;

  if (isAIDirty.value && !(await saveAISettings())) return;

  backfillingAITagLocalizations.value = true;
  try {
    const result = await runSettingsAction(backfillAITagLocalizations, {
      logLabel: "批量生成标签中文名失败:",
      fallbackErrorMessage: "无法创建标签中文翻译任务",
    });
    if (!result) return;

    aiSavedMessage.value = `已将 ${result.queued} 个标签加入中文翻译队列。${result.skipped > 0 ? ` ${result.skipped} 个标签已有翻译、无须翻译或任务已在队列中。` : ""}`;
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["ai-status"] }),
      queryClient.invalidateQueries({ queryKey: ["tags"] }),
    ]);
  } finally {
    backfillingAITagLocalizations.value = false;
  }
};

const handleReviewAITagSuggestion = async (
  suggestionId: string,
  payload: ReviewAITagSuggestionRequest,
) => {
  reviewingAITagSuggestionId.value = suggestionId;
  try {
    const result = await runSettingsAction(
      () => reviewAITagSuggestion(suggestionId, payload),
      {
        logLabel: "审核 AI 标签建议失败:",
        fallbackErrorMessage: "无法审核 AI 标签建议",
      },
    );
    if (!result) return;

    const action: AITagSuggestionReviewAction = payload.action;
    if (result.application) {
      recentTaggingRunIds.value = [
        result.suggestion.runId,
        ...recentTaggingRunIds.value.filter(
          (runId) => runId !== result.suggestion.runId,
        ),
      ];
    }
    aiSavedMessage.value =
      action === "reject"
        ? "已拒绝 AI 标签建议。"
        : `已应用标签 ${result.application?.namespace}:${result.application?.name}。`;
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["ai-tag-suggestions"] }),
      queryClient.invalidateQueries({ queryKey: ["archives"] }),
    ]);
  } finally {
    reviewingAITagSuggestionId.value = null;
  }
};

const handleUndoAITaggingRun = async (runId: string) => {
  const confirmed = await askForConfirmation({
    title: "撤销 AI 标签批次",
    message:
      "这会撤销该批次自动或人工应用的标签。批次应用前已存在或仍由其他 AI 批次保留的相同标签不会被删除。",
    type: "warning",
    confirmText: "撤销标签",
  });
  if (!confirmed) return;

  undoingAITaggingRunId.value = runId;
  try {
    const result = await runSettingsAction(() => undoAITaggingRun(runId), {
      logLabel: "撤销 AI 标签批次失败:",
      fallbackErrorMessage: "无法撤销 AI 标签批次",
    });
    if (!result) return;

    recentTaggingRunIds.value = recentTaggingRunIds.value.filter(
      (recentRunId) => recentRunId !== runId,
    );
    aiSavedMessage.value = `已撤销 ${result.applicationsUndone} 项应用，移除 ${result.archiveTagsRemoved} 个标签。`;
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["ai-tag-suggestions"] }),
      queryClient.invalidateQueries({ queryKey: ["archives"] }),
    ]);
  } finally {
    undoingAITaggingRunId.value = null;
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
    }
  } finally {
    installPluginLoading.value = false;
  }
};

const handleTogglePlugin = async (plugin: Plugin) => {
  const pluginId = plugin.plugin_id;

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

const addOperationRecord = (
  operation: string,
  success: boolean,
  result: string,
) => {
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
    addOperationRecord(
      "批量删除漫画",
      true,
      `删除了 ${archiveIds.length || "所有"} 个漫画`,
    );
    batchDeleteForm.value.archiveIds = "";
    await queryClient.invalidateQueries({ queryKey: ["archives"] });
  } catch (error) {
    console.error("批量删除漫画失败:", error);
    addOperationRecord(
      "批量删除漫画",
      false,
      getApiErrorMessage(error, "批量删除漫画失败"),
    );
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
      await showInfoDialog(
        "没有可删除内容",
        `分类 "${category.name}" 当前没有匹配的漫画。`,
      );
      return;
    }

    const categoryType =
      preview.categoryType === "dynamic" ? "动态分类" : "静态分类";
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
    const responseStatus = (error as { response?: { status?: number } })
      ?.response?.status;
    const errorMessage =
      responseStatus === 422
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
    const tagName = tag ? tagDisplayText(tag) : tagId;
    addOperationRecord(
      "按标签删除漫画",
      true,
      `删除了标签 \"${tagName}\" 下的所有漫画`,
    );
    batchDeleteForm.value.tagId = "";

    await queryClient.invalidateQueries({ queryKey: ["archives"] });
    await queryClient.invalidateQueries({ queryKey: ["tags"] });
  } catch (error) {
    console.error("按标签删除漫画失败:", error);
    addOperationRecord(
      "按标签删除漫画",
      false,
      getApiErrorMessage(error, "按标签删除漫画失败"),
    );
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
    addOperationRecord(
      "清理无用标签",
      false,
      getApiErrorMessage(error, "清理无用标签失败"),
    );
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
    addOperationRecord(
      "清理空分类",
      false,
      getApiErrorMessage(error, "清理空分类失败"),
    );
  } finally {
    batchOperationLoading.value = false;
  }
};

const handleManualScan = async () => {
  if (isSystemDirty.value && !(await saveSystemSettings())) return;

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

const loadAdminData = async () => {
  let loadedSystemSettings = false;
  let loadedScanSettings = false;
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
    loadedSystemSettings = true;
  } catch (error) {
    console.error("加载设置失败:", error);
  }

  try {
    aiSettings.value = normalizeLoadedAISettings(await getAISettings());
    markAISettingsSaved("已加载当前配置");
  } catch (error) {
    console.error("加载AI设置失败:", error);
  }

  try {
    const scanConfig = await getScanSettings();
    scanSettings.value = scanConfig.scanSettings;
    loadedScanSettings = true;
  } catch (error) {
    console.error("加载扫描设置失败:", error);
  }

  await loadCacheStatus();
  if (loadedSystemSettings && loadedScanSettings) {
    markSystemSettingsSaved("已加载当前配置");
  }
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

onBeforeRouteUpdate(async () => await confirmUnsavedSettings());
onBeforeRouteLeave(async () => await confirmUnsavedSettings());
</script>

<style scoped>
.settings-view {
  min-height: calc(100vh - 4rem);
}

.settings-panel-enter-active,
.settings-panel-leave-active {
  transition:
    opacity 180ms ease,
    transform 180ms ease;
}

.settings-panel-enter-from {
  opacity: 0;
  transform: translateY(4px);
}

.settings-panel-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}

@media (prefers-reduced-motion: reduce) {
  .settings-panel-enter-active,
  .settings-panel-leave-active {
    transition: none;
  }
}
</style>
