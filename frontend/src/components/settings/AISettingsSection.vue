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
      <div class="mb-4">
        <div>
          <h2 class="text-lg font-medium text-[var(--text-primary)]">
            AI 配置
          </h2>
          <p class="mt-1 text-sm text-[var(--text-secondary)]">
            连接和执行参数由所有 AI 功能共用。
          </p>
        </div>
      </div>

      <div class="space-y-4">
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

          <div>
            <label
              class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
              >模型</label
            >
            <input
              v-model.trim="aiSettings.connection.model"
              type="text"
              autocomplete="off"
              placeholder="例如 gpt-4o-mini"
              class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
            />
          </div>
        </div>

        <div>
          <label
            class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
            >Base URL</label
          >
          <input
            v-model.trim="aiSettings.connection.baseUrl"
            type="url"
            autocomplete="url"
            placeholder="https://api.openai.com/v1"
            class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
          />
        </div>

        <div>
          <label
            class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
            >API Key</label
          >
          <input
            v-model="aiSettings.connection.apiKey"
            type="password"
            autocomplete="new-password"
            :placeholder="apiKeyPlaceholder"
            class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
          />
          <p class="mt-1 text-xs text-[var(--text-secondary)]">
            {{ apiKeyHint }}
          </p>
        </div>

        <div class="flex flex-wrap gap-2">
          <GlassButton
            :disabled="aiLoading || !canTestConnection"
            :loading="testingConnection"
            loading-text="测试中..."
            variant="secondary"
            size="sm"
            @click="emit('test-connection')"
          >
            测试连接
          </GlassButton>
        </div>
      </div>
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
              v-if="!isKnownTargetLanguage(aiSettings.features.titleTranslation.targetLanguage)"
              :value="aiSettings.features.titleTranslation.targetLanguage"
            >
              当前配置（{{ aiSettings.features.titleTranslation.targetLanguage }}）
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

    <GlassCard size="md" radius="lg">
      <h2 class="mb-4 text-lg font-medium text-[var(--text-primary)]">
        AI 自动标签
      </h2>
      <div class="space-y-4">
        <label
          class="flex items-center gap-2 text-sm text-[var(--text-primary)]"
        >
          <input
            v-model="aiSettings.features.autoTagging.enabled"
            type="checkbox"
            class="rounded"
          />
          启用 AI 自动标签
        </label>

        <div>
          <label
            class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
            >自动应用阈值</label
          >
          <div class="flex items-center gap-3">
            <input
              v-model.number="
                aiSettings.features.autoTagging.autoApplyThreshold
              "
              type="range"
              min="0.1"
              max="1"
              step="0.1"
              class="flex-1"
            />
            <span class="w-12 text-sm text-[var(--text-primary)]"
              >{{
                (
                  aiSettings.features.autoTagging.autoApplyThreshold * 100
                ).toFixed(0)
              }}%</span
            >
          </div>
        </div>
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
      </div>
    </GlassCard>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { ArrowPathIcon } from "@heroicons/vue/24/outline";
import GlassButton from "@/components/base/GlassButton.vue";
import GlassCard from "@/components/base/GlassCard.vue";
import SettingsSaveBar from "@/components/settings/SettingsSaveBar.vue";
import type { AISettings, AIStatus } from "@/types/api";

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

const canTestConnection = computed(() => {
  const { baseUrl, model, apiKey, apiKeyConfigured } =
    props.aiSettings.connection;
  return Boolean(
    baseUrl.trim() && model.trim() && (apiKey?.trim() || apiKeyConfigured),
  );
});

const apiKeyPlaceholder = computed(() =>
  props.aiSettings.connection.apiKeyConfigured
    ? "已配置。留空时保留现有密钥"
    : "输入 API Key",
);

const apiKeyHint = computed(() =>
  props.aiSettings.connection.apiKeyConfigured
    ? "密钥已配置，保存时留空将继续使用现有密钥。"
    : "密钥仅在保存或测试连接时发送，不会在此页面回显。",
);
</script>
