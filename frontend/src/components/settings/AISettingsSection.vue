<template>
  <div class="space-y-6">
    <SettingsSaveBar
      :dirty="aiDirty"
      :saving="aiLoading"
      :saved-message="savedMessage"
      @save="emit('save')"
      @discard="emit('discard')"
    />

    <GlassCard size="md" radius="lg">
      <div class="mb-4 flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 class="text-lg font-medium text-[var(--text-primary)]">AI 连接</h2>
          <p class="mt-1 text-sm text-[var(--text-secondary)]">
            标题翻译会保留入队时的配置；内容分析会在执行时使用当前启用的配置。
          </p>
        </div>
        <GlassButton variant="secondary" size="sm" @click="addProfile">
          添加配置
        </GlassButton>
      </div>

      <div class="mb-5 flex flex-wrap gap-2">
        <button
          v-for="profile in aiSettings.profiles"
          :key="profile.id"
          type="button"
          class="rounded-lg border px-3 py-2 text-left text-sm transition-colors"
          :class="
            profile.id === aiSettings.activeProfileId
              ? 'border-[var(--accent)] bg-[var(--accent)]/10 text-[var(--text-primary)]'
              : 'border-[var(--border)] bg-[var(--bg-tertiary)] text-[var(--text-secondary)] hover:text-[var(--text-primary)]'
          "
          @click="aiSettings.activeProfileId = profile.id"
        >
          <span class="block font-medium">{{
            profile.name || "未命名配置"
          }}</span>
          <span class="block text-xs opacity-75">{{
            profile.enabled ? "已启用" : "已停用"
          }}</span>
        </button>
      </div>

      <div v-if="activeProfile" class="space-y-4">
        <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <div>
            <label
              class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
              >配置名称</label
            >
            <input
              v-model.trim="activeProfile.name"
              type="text"
              autocomplete="off"
              placeholder="例如 Ollama 本地"
              class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
            />
          </div>

          <div>
            <label
              class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
              >模型</label
            >
            <input
              v-model.trim="activeProfile.connection.model"
              type="text"
              autocomplete="off"
              placeholder="例如 gpt-4o-mini 或 qwen3:8b"
              class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
            />
          </div>
        </div>

        <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <div>
            <label
              class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
              >服务协议</label
            >
            <div
              class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-sm text-[var(--text-primary)]"
            >
              OpenAI-compatible Chat Completions
            </div>
          </div>
          <label
            class="flex items-center gap-2 self-end pb-2 text-sm text-[var(--text-primary)]"
          >
            <input
              v-model="activeProfile.enabled"
              type="checkbox"
              class="rounded"
              :disabled="!canToggleActiveProfile"
              @change="switchFromDisabledActiveProfile"
            />
            启用此配置
          </label>
        </div>

        <div>
          <label
            class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
            >Base URL</label
          >
          <input
            v-model.trim="activeProfile.connection.baseUrl"
            type="url"
            autocomplete="url"
            placeholder="https://api.openai.com/v1 或 http://localhost:11434/v1"
            class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
          />
        </div>

        <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <div>
            <label
              class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
              >认证方式</label
            >
            <select
              v-model="activeProfile.connection.authMode"
              class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
            >
              <option value="bearer">Bearer API Key</option>
              <option value="none">无认证</option>
            </select>
          </div>
          <div v-if="activeProfile.connection.authMode === 'bearer'">
            <label
              class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
              >API Key</label
            >
            <input
              v-model="activeProfile.connection.apiKey"
              type="password"
              autocomplete="new-password"
              :placeholder="apiKeyPlaceholder"
              class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
            />
            <p class="mt-1 text-xs text-[var(--text-secondary)]">
              {{ apiKeyHint }}
            </p>
          </div>
        </div>

        <div class="flex flex-wrap gap-2">
          <GlassButton
            :disabled="
              aiLoading || !canTestConnection || !activeProfile.enabled
            "
            :loading="testingConnection"
            loading-text="测试中..."
            variant="secondary"
            size="sm"
            @click="emit('test-connection')"
          >
            测试视觉连接
          </GlassButton>
          <GlassButton
            :disabled="aiLoading || aiSettings.profiles.length === 1"
            variant="danger"
            size="sm"
            @click="removeActiveProfile"
          >
            删除配置
          </GlassButton>
        </div>
      </div>
    </GlassCard>

    <GlassCard size="md" radius="lg">
      <div class="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 class="text-lg font-medium text-[var(--text-primary)]">内容分析</h2>
          <p class="mt-1 text-sm text-[var(--text-secondary)]">
            每本新漫画入库后会在后台抽取封面和 8 到 20 张正文页，识别题材与内容特征，为随机精选和偏好规则提供依据。
          </p>
        </div>
        <span class="rounded-md border border-[var(--border)] bg-[var(--bg-tertiary)] px-2 py-1 text-xs text-[var(--text-secondary)]">异步执行</span>
      </div>
      <dl class="mt-5 divide-y divide-[var(--border)] border-y border-[var(--border)]">
        <div class="grid grid-cols-[116px_minmax(0,1fr)] gap-4 py-3 text-sm">
          <dt class="font-medium text-[var(--text-primary)]">模型要求</dt>
          <dd class="text-[var(--text-secondary)]">必须选择支持图片输入的视觉模型。仅支持文本的模型可以用于标题翻译，但无法完成内容分析。</dd>
        </div>
        <div class="grid grid-cols-[116px_minmax(0,1fr)] gap-4 py-3 text-sm">
          <dt class="font-medium text-[var(--text-primary)]">何时生效</dt>
          <dd class="text-[var(--text-secondary)]">扫描新漫画后自动入队，不阻塞入库或阅读；分析失败时保留漫画并在后台重试。</dd>
        </div>
        <div class="grid grid-cols-[116px_minmax(0,1fr)] gap-4 py-3 text-sm">
          <dt class="font-medium text-[var(--text-primary)]">OCR 辅助</dt>
          <dd class="text-[var(--text-secondary)]">可在“OCR 辅助”中启用本地文字识别，为页面分析补充文字线索；它不提供阅读器翻译或全库文字搜索。</dd>
        </div>
      </dl>
    </GlassCard>

    <GlassCard size="md" radius="lg">
      <h2 class="mb-4 text-lg font-medium text-[var(--text-primary)]">
        执行参数
      </h2>
      <div class="grid grid-cols-1 gap-4 sm:grid-cols-3">
        <div>
          <label
            class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
            >最大并发任务数</label
          >
          <input
            v-model.number="aiSettings.execution.maxConcurrentTasks"
            type="number"
            min="1"
            max="10"
            class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
          />
        </div>

        <div>
          <label
            class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
            >请求超时（秒）</label
          >
          <input
            v-model.number="aiSettings.execution.timeoutSeconds"
            type="number"
            min="10"
            max="1800"
            class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
          />
        </div>

        <div>
          <label
            class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
            >最大重试次数</label
          >
          <input
            v-model.number="aiSettings.execution.maxRetries"
            type="number"
            min="0"
            max="10"
            class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
          />
        </div>
      </div>
    </GlassCard>

    <GlassCard size="md" radius="lg">
      <div class="mb-4 flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 class="text-lg font-medium text-[var(--text-primary)]">
            标题翻译
          </h2>
          <p class="mt-1 text-sm text-[var(--text-secondary)]">
            译文作为副标题显示，原始标题保持不变。
          </p>
        </div>
        <div class="flex flex-wrap gap-2">
          <GlassButton
            :disabled="
              aiLoading ||
              repairingTranslations ||
              retranslatingTranslations ||
              !aiSettings.features.titleTranslation.enabled
            "
            :loading="backfillingTranslations"
            loading-text="加入队列中..."
            variant="secondary"
            size="sm"
            @click="emit('backfill-title-translations')"
          >
            {{ aiDirty ? "保存并加入队列" : "批量补翻译" }}
          </GlassButton>
          <GlassButton
            :disabled="
              aiLoading ||
              backfillingTranslations ||
              retranslatingTranslations ||
              !aiSettings.features.titleTranslation.enabled
            "
            :loading="repairingTranslations"
            loading-text="筛选并入队中..."
            variant="secondary"
            size="sm"
            @click="emit('repair-suspicious-title-translations')"
          >
            修复失败/疑似拒答
          </GlassButton>
          <GlassButton
            :disabled="
              aiLoading ||
              backfillingTranslations ||
              repairingTranslations ||
              !aiSettings.features.titleTranslation.enabled
            "
            :loading="retranslatingTranslations"
            loading-text="重新入队中..."
            variant="secondary"
            size="sm"
            @click="emit('force-retranslate-title-translations')"
          >
            <template #icon>
              <ArrowPathIcon class="mr-1.5 h-4 w-4" />
            </template>
            全部重新翻译
          </GlassButton>
        </div>
      </div>

      <div class="space-y-4">
        <label
          class="flex items-center gap-2 text-sm text-[var(--text-primary)]"
        >
          <input
            v-model="aiSettings.features.titleTranslation.enabled"
            type="checkbox"
            class="rounded"
          />
          自动翻译刮削后的标题
        </label>

        <label
          class="flex items-start gap-2 text-sm text-[var(--text-primary)]"
        >
          <input
            v-model="aiSettings.features.titleTranslation.displayTranslatedTitle"
            type="checkbox"
            class="mt-0.5 rounded"
          />
          <span>
            <span class="block">主标题显示译文</span>
            <span class="mt-1 block text-xs text-[var(--text-secondary)]">
              有可用译文时，译文显示为主标题，原始标题显示为副标题；没有译文时自动回退为原始标题。
            </span>
          </span>
        </label>

        <div class="max-w-sm">
          <label
            class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
            >目标语言</label
          >
          <select
            v-model="aiSettings.features.titleTranslation.targetLanguage"
            class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
          >
            <option
              v-if="
                !isKnownTargetLanguage(
                  aiSettings.features.titleTranslation.targetLanguage,
                )
              "
              :value="aiSettings.features.titleTranslation.targetLanguage"
            >
              当前配置（{{
                aiSettings.features.titleTranslation.targetLanguage
              }}）
            </option>
            <option
              v-for="language in titleTranslationLanguages"
              :key="language.code"
              :value="language.code"
            >
              {{ language.label }}
            </option>
          </select>
        </div>

        <label
          class="flex items-center gap-2 text-sm text-[var(--text-primary)]"
        >
          <input
            v-model="aiSettings.features.titleTranslation.skipIfTargetLanguage"
            type="checkbox"
            class="rounded"
          />
          跳过已为目标语言的标题
        </label>

        <label
          class="flex items-center gap-2 text-sm text-[var(--text-primary)]"
        >
          <input
            v-model="
              aiSettings.features.titleTranslation.retranslateOnTitleChange
            "
            type="checkbox"
            class="rounded"
          />
          原始标题变更后重新翻译
        </label>
      </div>
    </GlassCard>

    <GlassCard v-if="aiStatus" size="md" radius="lg">
      <h2 class="mb-3 text-lg font-medium text-[var(--text-primary)]">
        运行状态
      </h2>
      <div class="grid grid-cols-2 gap-3 md:grid-cols-4">
        <div
          class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3 text-center"
        >
          <div class="text-xl font-semibold text-[var(--accent)]">
            {{ aiStatus.queueSize }}
          </div>
          <div class="text-xs text-[var(--text-secondary)]">队列中</div>
        </div>
        <div
          class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3 text-center"
        >
          <div class="text-xl font-semibold text-green-500">
            {{ aiStatus.processingCount }}
          </div>
          <div class="text-xs text-[var(--text-secondary)]">处理中</div>
        </div>
        <div
          class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3 text-center"
        >
          <div class="text-xl font-semibold text-[var(--accent)]">
            {{ aiStatus.completedToday }}
          </div>
          <div class="text-xs text-[var(--text-secondary)]">今日完成</div>
        </div>
        <div
          class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3 text-center"
        >
          <div class="text-xl font-semibold text-red-500">
            {{ aiStatus.failedToday }}
          </div>
          <div class="text-xs text-[var(--text-secondary)]">今日失败</div>
        </div>
        <div
          class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3 text-center"
        >
          <div class="text-xl font-semibold text-[var(--accent)]">
            {{ aiStatus.languageDetectionPending }}
          </div>
          <div class="text-xs text-[var(--text-secondary)]">待语言确认</div>
        </div>
        <div
          class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3 text-center"
        >
          <div class="text-xl font-semibold text-amber-500">
            {{ aiStatus.retryScheduled }}
          </div>
          <div class="text-xs text-[var(--text-secondary)]">等待重试</div>
        </div>
        <div
          class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3 text-center"
        >
          <div class="text-xl font-semibold text-red-500">
            {{ aiStatus.unresolvedFailureCount }}
          </div>
          <div class="text-xs text-[var(--text-secondary)]">待处理失败</div>
        </div>
      </div>
      <p
        v-if="aiStatus.providerBlockedUntil"
        class="mt-3 text-sm text-amber-600 dark:text-amber-400"
      >
        AI 服务限流中，{{
          new Date(aiStatus.providerBlockedUntil).toLocaleString()
        }}
        后自动恢复。
      </p>
    </GlassCard>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { ArrowPathIcon } from "@heroicons/vue/24/outline";
import GlassButton from "@/components/base/GlassButton.vue";
import GlassCard from "@/components/base/GlassCard.vue";
import SettingsSaveBar from "@/components/settings/SettingsSaveBar.vue";
import type { AIConnectionProfile, AISettings, AIStatus } from "@/types/api";

interface Props {
  aiSettings: AISettings;
  aiStatus?: AIStatus;
  aiLoading: boolean;
  aiDirty: boolean;
  savedMessage: string | null;
  testingConnection: boolean;
  backfillingTranslations: boolean;
  repairingTranslations: boolean;
  retranslatingTranslations: boolean;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  save: [];
  discard: [];
  "test-connection": [];
  "backfill-title-translations": [];
  "repair-suspicious-title-translations": [];
  "force-retranslate-title-translations": [];
}>();

const titleTranslationLanguages = [
  { code: "zh-CN", label: "简体中文（zh-CN）" },
  { code: "zh-TW", label: "繁体中文（zh-TW）" },
  { code: "ja", label: "日语（ja）" },
  { code: "ko", label: "韩语（ko）" },
  { code: "en", label: "英语（en）" },
  { code: "fr", label: "法语（fr）" },
  { code: "de", label: "德语（de）" },
  { code: "es", label: "西班牙语（es）" },
  { code: "pt", label: "葡萄牙语（pt）" },
  { code: "it", label: "意大利语（it）" },
  { code: "ru", label: "俄语（ru）" },
  { code: "uk", label: "乌克兰语（uk）" },
] as const;

const isKnownTargetLanguage = (language: string) =>
  titleTranslationLanguages.some((option) => option.code === language);

const activeProfile = computed(() =>
  props.aiSettings.profiles.find(
    (profile) => profile.id === props.aiSettings.activeProfileId,
  ),
);

const newProfileId = () =>
  globalThis.crypto?.randomUUID?.() ??
  `profile-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;

const addProfile = () => {
  const profile: AIConnectionProfile = {
    id: newProfileId(),
    name: "新 AI 配置",
    enabled: true,
    connection: {
      provider: "openaiCompatible",
      baseUrl: "http://localhost:11434/v1",
      model: "",
      authMode: "none",
      apiKeyConfigured: false,
    },
  };
  props.aiSettings.profiles.push(profile);
  props.aiSettings.activeProfileId = profile.id;
};

const removeActiveProfile = () => {
  const profile = activeProfile.value;
  if (!profile || props.aiSettings.profiles.length <= 1) return;
  const index = props.aiSettings.profiles.findIndex(
    (item) => item.id === profile.id,
  );
  props.aiSettings.profiles.splice(index, 1);
  props.aiSettings.activeProfileId = props.aiSettings.profiles[0].id;
};

const canToggleActiveProfile = computed(
  () =>
    activeProfile.value?.enabled === false ||
    props.aiSettings.profiles.filter((profile) => profile.enabled).length > 1,
);

const switchFromDisabledActiveProfile = () => {
  const profile = activeProfile.value;
  if (!profile || profile.enabled) return;
  const nextProfile = props.aiSettings.profiles.find(
    (item) => item.enabled && item.id !== profile.id,
  );
  if (nextProfile) props.aiSettings.activeProfileId = nextProfile.id;
};

const canTestConnection = computed(() => {
  const connection = activeProfile.value?.connection;
  if (!connection) return false;
  const { baseUrl, model, apiKey, apiKeyConfigured, authMode } = connection;
  return Boolean(
    baseUrl.trim() &&
    model.trim() &&
    (authMode === "none" || apiKey?.trim() || apiKeyConfigured),
  );
});

const apiKeyPlaceholder = computed(() =>
  activeProfile.value?.connection.apiKeyConfigured
    ? "已配置。留空时保留现有密钥"
    : "输入 API Key",
);

const apiKeyHint = computed(() =>
  activeProfile.value?.connection.apiKeyConfigured
    ? "密钥已配置，保存时留空将继续使用现有密钥。"
    : "密钥仅在保存或测试连接时发送，不会在此页面回显。",
);
</script>
