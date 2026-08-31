<template>
  <div class="space-y-6">
    <SettingsSaveBar
      :dirty="isDirty"
      :saving="saving"
      :saved-message="message"
      :error="error"
      :error-title="errorTitle"
      @save="save"
      @discard="discard"
    />

    <GlassCard size="md" radius="lg">
      <div class="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h2 class="text-lg font-medium text-[var(--text-primary)]">
            Embedding 模型
          </h2>
          <p class="mt-1 max-w-2xl text-sm text-[var(--text-secondary)]">
            配置标签语义向量的服务。后续标签聚类只会使用这里配置的 embedding
            接口，不会调用聊天模型接口。
          </p>
        </div>
        <span
          class="rounded-md border px-2 py-1 text-xs"
          :class="
            settings.apiKeyConfigured || settings.authMode === 'none'
              ? 'border-green-500/40 bg-green-500/10 text-green-700 dark:text-green-300'
              : 'border-amber-500/40 bg-amber-500/10 text-amber-700 dark:text-amber-300'
          "
        >
          {{
            settings.authMode === "none"
              ? "无需认证"
              : settings.apiKeyConfigured
                ? "API Key 已配置"
                : "需要 API Key"
          }}
        </span>
      </div>

      <div class="mt-6 grid grid-cols-1 gap-4 sm:grid-cols-2">
        <div>
          <label
            class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
            for="embedding-provider"
          >
            服务协议
          </label>
          <select
            id="embedding-provider"
            v-model="settings.provider"
            :disabled="formDisabled"
            class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
          >
            <option value="ollama">Ollama 原生 Embedding API</option>
            <option value="openaiCompatible">
              OpenAI-compatible Embeddings API
            </option>
          </select>
        </div>

        <div>
          <label
            class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
            for="embedding-model"
          >
            模型名称
          </label>
          <input
            id="embedding-model"
            v-model.trim="settings.model"
            type="text"
            autocomplete="off"
            placeholder="输入 embedding 模型名称"
            :disabled="formDisabled"
            class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
          />
        </div>
      </div>

      <div class="mt-4">
        <label
          class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
          for="embedding-base-url"
        >
          Base URL
        </label>
        <input
          id="embedding-base-url"
          v-model.trim="settings.baseUrl"
          type="url"
          autocomplete="url"
          :placeholder="baseUrlPlaceholder"
          :disabled="formDisabled"
          class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
        />
        <p class="mt-1 break-all text-xs text-[var(--text-secondary)]">
          输入服务根地址，实际请求路径为 {{ endpointPreview }}。
        </p>
      </div>

      <div class="mt-4 grid grid-cols-1 gap-4 sm:grid-cols-2">
        <div>
          <label
            class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
            for="embedding-auth-mode"
          >
            认证方式
          </label>
          <select
            id="embedding-auth-mode"
            v-model="settings.authMode"
            :disabled="formDisabled"
            class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
          >
            <option value="none">无认证</option>
            <option value="bearer">Bearer API Key</option>
          </select>
        </div>

        <div v-if="settings.authMode === 'bearer'">
          <label
            class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
            for="embedding-api-key"
          >
            API Key
          </label>
          <input
            id="embedding-api-key"
            v-model="settings.apiKey"
            type="password"
            autocomplete="new-password"
            :disabled="formDisabled"
            :placeholder="
              settings.apiKeyConfigured
                ? '已配置，留空则继续使用当前 key'
                : '输入 API Key'
            "
            class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
          />
          <p class="mt-1 text-xs text-[var(--text-secondary)]">
            key 只保存在服务端，不会随读取配置返回。
          </p>
        </div>
      </div>

      <div class="mt-4 grid grid-cols-1 gap-4 sm:grid-cols-3">
        <div>
          <label
            class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
            for="embedding-timeout"
          >
            请求超时（秒）
          </label>
          <input
            id="embedding-timeout"
            v-model.number="settings.timeoutSeconds"
            type="number"
            min="5"
            max="3600"
            step="1"
            :disabled="formDisabled"
            class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
          />
        </div>

        <div>
          <label
            class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
            for="embedding-interval"
          >
            请求间隔（秒）
          </label>
          <input
            id="embedding-interval"
            v-model.number="settings.requestIntervalSeconds"
            type="number"
            min="0"
            max="3600"
            step="1"
            :disabled="formDisabled"
            class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
          />
          <p class="mt-1 text-xs text-[var(--text-secondary)]">
            连续批量请求之间的最小等待时间；本地模型建议保留非零值。
          </p>
        </div>

        <div>
          <label
            class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
            for="embedding-dimensions"
          >
            目标维度（可选）
          </label>
          <input
            id="embedding-dimensions"
            v-model.number="settings.dimensions"
            type="number"
            min="1"
            max="65536"
            step="1"
            :disabled="formDisabled || settings.provider === 'ollama'"
            placeholder="由模型决定"
            class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] disabled:cursor-not-allowed disabled:opacity-60"
          />
          <p class="mt-1 text-xs text-[var(--text-secondary)]">
            仅 OpenAI-compatible 接口会发送该参数；Ollama 使用模型原生维度。
          </p>
        </div>
      </div>

      <div
        class="mt-6 flex flex-wrap items-center gap-3 border-t border-[var(--border)] pt-5"
      >
        <GlassButton
          variant="secondary"
          size="sm"
          :disabled="formDisabled"
          :loading="testing"
          loading-text="测试中..."
          @click="testConnection"
        >
          测试连接
        </GlassButton>
        <GlassButton
          v-if="loadState === 'error'"
          variant="ghost"
          size="sm"
          :disabled="loading"
          @click="load"
        >
          重试读取
        </GlassButton>
        <span v-else-if="loading" class="text-xs text-[var(--text-secondary)]">
          正在读取配置...
        </span>
      </div>
    </GlassCard>

    <GlassCard size="md" radius="lg">
      <h2 class="text-lg font-medium text-[var(--text-primary)]">当前接口</h2>
      <dl
        class="mt-4 divide-y divide-[var(--border)] border-y border-[var(--border)]"
      >
        <div class="grid grid-cols-[116px_minmax(0,1fr)] gap-4 py-3 text-sm">
          <dt class="font-medium text-[var(--text-primary)]">请求格式</dt>
          <dd class="text-[var(--text-secondary)]">
            {{
              settings.provider === "ollama"
                ? "POST /api/embed，读取 embeddings"
                : "POST /embeddings，读取 data[].embedding"
            }}
          </dd>
        </div>
        <div class="grid grid-cols-[116px_minmax(0,1fr)] gap-4 py-3 text-sm">
          <dt class="font-medium text-[var(--text-primary)]">使用场景</dt>
          <dd class="text-[var(--text-secondary)]">
            只为非 metadata namespace 的标签生成向量；metadata namespace
            会在后续聚类任务中排除。
          </dd>
        </div>
        <div class="grid grid-cols-[116px_minmax(0,1fr)] gap-4 py-3 text-sm">
          <dt class="font-medium text-[var(--text-primary)]">数据策略</dt>
          <dd class="text-[var(--text-secondary)]">
            新配置只影响后续逐步处理，不会触发全库回填，也不会直接创建偏好规则。
          </dd>
        </div>
      </dl>
    </GlassCard>
  </div>
</template>

<script setup lang="ts">
import axios from "axios";
import { computed, onMounted, ref, watch } from "vue";
import GlassButton from "@/components/base/GlassButton.vue";
import GlassCard from "@/components/base/GlassCard.vue";
import SettingsSaveBar from "@/components/settings/SettingsSaveBar.vue";
import {
  getEmbeddingSettings,
  testEmbeddingConnection,
  updateEmbeddingSettings,
} from "@/utils/api";
import type { EmbeddingSettings } from "@/types/api";

const emit = defineEmits<{
  "dirty-change": [dirty: boolean];
}>();

const defaultSettings = (): EmbeddingSettings => ({
  settingsVersion: 1,
  provider: "ollama",
  baseUrl: "http://localhost:11434",
  model: "",
  timeoutSeconds: 120,
  requestIntervalSeconds: 20,
  dimensions: null,
  authMode: "none",
  apiKey: "",
  apiKeyConfigured: false,
});

const cloneSettings = (value: EmbeddingSettings): EmbeddingSettings =>
  JSON.parse(JSON.stringify(value)) as EmbeddingSettings;

const normalizeSettings = (
  value: Partial<EmbeddingSettings>,
): EmbeddingSettings => ({
  ...defaultSettings(),
  ...value,
  baseUrl: value.baseUrl?.trim() || defaultSettings().baseUrl,
  model: value.model?.trim() || defaultSettings().model,
  dimensions:
    value.dimensions == null || Number(value.dimensions) <= 0
      ? null
      : Number(value.dimensions),
  apiKey: "",
  apiKeyConfigured: Boolean(value.apiKeyConfigured),
});

const settings = ref<EmbeddingSettings>(defaultSettings());
const savedSettings = ref<EmbeddingSettings | null>(null);
const loading = ref(true);
const loadState = ref<"loading" | "ready" | "error">("loading");
const saving = ref(false);
const testing = ref(false);
const message = ref<string | null>(null);
const error = ref<string | null>(null);
const errorTitle = ref("保存失败");

const isDirty = computed(
  () =>
    loadState.value === "ready" &&
    savedSettings.value !== null &&
    JSON.stringify(settings.value) !== JSON.stringify(savedSettings.value),
);

const formDisabled = computed(
  () =>
    loading.value ||
    saving.value ||
    testing.value ||
    loadState.value !== "ready",
);

const baseUrlPlaceholder = computed(() =>
  settings.value.provider === "ollama"
    ? "http://localhost:11434"
    : "https://api.openai.com/v1",
);

const endpointPreview = computed(() => {
  const base = settings.value.baseUrl.trim().replace(/\/+$/, "");
  if (!base) {
    return settings.value.provider === "ollama"
      ? "{baseUrl}/api/embed"
      : "{baseUrl}/embeddings";
  }
  if (settings.value.provider === "ollama") {
    if (base.endsWith("/api/embed")) return base;
    if (base.endsWith("/api")) return base + "/embed";
    return base + "/api/embed";
  }
  return base.endsWith("/embeddings") ? base : base + "/embeddings";
});

const requestPayload = (): EmbeddingSettings => ({
  ...settings.value,
  baseUrl: settings.value.baseUrl.trim(),
  model: settings.value.model.trim(),
  dimensions:
    settings.value.dimensions == null || Number(settings.value.dimensions) <= 0
      ? null
      : Number(settings.value.dimensions),
  apiKey: settings.value.apiKey?.trim() || undefined,
});

const errorMessage = (value: unknown, fallback: string) => {
  if (axios.isAxiosError(value)) {
    const data: unknown = value.response?.data;
    if (typeof data === "object" && data !== null) {
      const responseMessage = (data as { message?: unknown }).message;
      if (typeof responseMessage === "string" && responseMessage.trim()) {
        return responseMessage;
      }
    }
  }
  return value instanceof Error && value.message ? value.message : fallback;
};

const load = async (): Promise<boolean> => {
  loading.value = true;
  loadState.value = "loading";
  error.value = null;
  errorTitle.value = "读取失败";
  try {
    const loaded = normalizeSettings(await getEmbeddingSettings());
    settings.value = loaded;
    savedSettings.value = cloneSettings(loaded);
    loadState.value = "ready";
    message.value = "已加载当前 embedding 配置";
    return true;
  } catch (value) {
    loadState.value = "error";
    error.value = errorMessage(value, "无法读取 embedding 配置");
    return false;
  } finally {
    loading.value = false;
  }
};

const save = async (): Promise<boolean> => {
  if (loadState.value !== "ready") return false;
  saving.value = true;
  error.value = null;
  errorTitle.value = "保存失败";
  try {
    await updateEmbeddingSettings(requestPayload());
    if (!(await load())) return false;
    message.value = "Embedding 配置已保存";
    return true;
  } catch (value) {
    error.value = errorMessage(value, "保存 embedding 配置失败");
    return false;
  } finally {
    saving.value = false;
  }
};

const discard = () => {
  if (!savedSettings.value) return;
  settings.value = cloneSettings(savedSettings.value);
  message.value = "已放弃未保存的 embedding 配置";
  error.value = null;
};

const testConnection = async () => {
  if (loadState.value !== "ready") return;
  testing.value = true;
  error.value = null;
  message.value = null;
  errorTitle.value = "连接测试失败";
  try {
    const result = await testEmbeddingConnection(requestPayload());
    if (result.success) {
      message.value = "Embedding 连接测试成功，已收到有效向量";
    } else {
      error.value = result.message || "Embedding 连接测试失败";
    }
  } catch (value) {
    error.value = errorMessage(value, "Embedding 连接测试失败");
  } finally {
    testing.value = false;
  }
};

watch(isDirty, (dirty) => emit("dirty-change", dirty), { immediate: true });

defineExpose({ save, discard });

onMounted(() => void load());
</script>
