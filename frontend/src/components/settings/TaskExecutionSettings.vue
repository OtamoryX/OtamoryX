<template>
  <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
    <div>
      <label class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
        >模型配置</label
      >
      <select
        :value="execution.profileId"
        class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
        @change="
          update({ profileId: ($event.target as HTMLSelectElement).value })
        "
      >
        <option value="auto">自动选择兼容模型</option>
        <option
          v-for="profile in profiles"
          :key="profile.id"
          :value="profile.id"
          :disabled="!profile.enabled"
        >
          {{ profile.name
          }}{{
            showVisionCapability
              ? profile.connection.visionCapable
                ? "（视觉）"
                : "（文本）"
              : ""
          }}
        </option>
      </select>
    </div>

    <div>
      <label class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
        >温度</label
      >
      <input
        :value="execution.temperature"
        type="number"
        min="0"
        max="2"
        step="0.05"
        class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
        @input="update({ temperature: numberValue($event) })"
      />
    </div>

    <div>
      <label class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
        >结构化输出</label
      >
      <select
        :value="execution.structuredOutputMode"
        class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
        @change="
          update({
            structuredOutputMode: ($event.target as HTMLSelectElement)
              .value as AITaskExecutionSettings['structuredOutputMode'],
          })
        "
      >
        <option value="jsonObject">JSON object</option>
        <option v-if="allowJsonSchema" value="jsonSchema">JSON Schema</option>
        <option value="promptOnly">仅提示词</option>
      </select>
    </div>

    <div>
      <label class="mb-2 block text-sm font-medium text-[var(--text-primary)]"
        >思考模式</label
      >
      <select
        :value="execution.thinkingMode"
        class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
        @change="
          update({
            thinkingMode: ($event.target as HTMLSelectElement)
              .value as AITaskExecutionSettings['thinkingMode'],
          })
        "
      >
        <option value="inherit">沿用模型配置（默认）</option>
        <option value="disabled">关闭</option>
        <option value="enabled">开启</option>
      </select>
    </div>

    <div class="sm:col-span-2">
      <label
        class="mb-2 flex items-center gap-2 text-sm font-medium text-[var(--text-primary)]"
      >
        <input
          :checked="hasOutputOverride"
          type="checkbox"
          class="rounded"
          @change="
            setOutputOverride(($event.target as HTMLInputElement).checked)
          "
        />
        自定义输出 token
      </label>
      <div
        v-if="hasOutputOverride"
        class="grid grid-cols-1 gap-3 sm:grid-cols-2"
      >
        <label class="block text-xs text-[var(--text-secondary)]">
          关闭思考
          <input
            :value="execution.outputTokenLimit ?? defaults.outputTokenLimit"
            type="number"
            min="32"
            max="32768"
            class="mt-1 w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-sm text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
            @input="update({ outputTokenLimit: numberValue($event) })"
          />
        </label>
        <label class="block text-xs text-[var(--text-secondary)]">
          开启思考
          <input
            :value="
              execution.thinkingOutputTokenLimit ??
              defaults.thinkingOutputTokenLimit
            "
            type="number"
            min="32"
            max="32768"
            class="mt-1 w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-sm text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
            @input="update({ thinkingOutputTokenLimit: numberValue($event) })"
          />
        </label>
      </div>
      <p v-else class="pt-2 text-xs text-[var(--text-secondary)]">
        沿用全局默认：关闭思考 {{ defaults.outputTokenLimit }}，开启思考
        {{ defaults.thinkingOutputTokenLimit }}。
      </p>
      <p class="pt-2 text-xs text-[var(--text-secondary)]">
        这里的值是该任务首次请求的基础输出预算。明确需要恢复时，系统可能仅对当前重试临时提高到基础值的两倍；实际值不会超过可用上下文和模型限制，也不会写回此配置。
      </p>
    </div>

    <div>
      <label
        class="mb-2 flex items-center gap-2 text-sm font-medium text-[var(--text-primary)]"
      >
        <input
          :checked="execution.timeoutSeconds !== null"
          type="checkbox"
          class="rounded"
          @change="
            setTimeoutOverride(($event.target as HTMLInputElement).checked)
          "
        />
        自定义超时
      </label>
      <input
        :value="execution.timeoutSeconds ?? defaults.timeoutSeconds"
        :disabled="execution.timeoutSeconds === null"
        type="number"
        min="5"
        max="3600"
        class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] disabled:cursor-not-allowed disabled:opacity-60"
        @input="update({ timeoutSeconds: numberValue($event) })"
      />
    </div>

    <div>
      <label
        class="mb-2 flex items-center gap-2 text-sm font-medium text-[var(--text-primary)]"
      >
        <input
          :checked="execution.firstTokenTimeoutSeconds !== null"
          type="checkbox"
          class="rounded"
          @change="
            setFirstTokenOverride(($event.target as HTMLInputElement).checked)
          "
        />
        首 token 超时
      </label>
      <input
        :value="execution.firstTokenTimeoutSeconds ?? resolvedFirstTokenDefault"
        :disabled="execution.firstTokenTimeoutSeconds === null"
        type="number"
        min="1"
        :max="effectiveTimeout"
        class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-sm text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] disabled:cursor-not-allowed disabled:opacity-60"
        @input="update({ firstTokenTimeoutSeconds: numberValue($event) })"
      />
      <p class="pt-2 text-xs text-[var(--text-secondary)]">
        等待首个生成 token 的时限，不可超过该任务的整体请求超时
        {{ effectiveTimeout }} 秒。沿用所选模型配置：{{
          resolvedFirstTokenDefault
        }} 秒。
      </p>
    </div>

    <div>
      <label
        class="mb-2 flex items-center gap-2 text-sm font-medium text-[var(--text-primary)]"
      >
        <input
          :checked="execution.thinkingContextWindowTokens !== null"
          type="checkbox"
          class="rounded"
          @change="
            setThinkingContextOverride(
              ($event.target as HTMLInputElement).checked,
            )
          "
        />
        思考时使用独立上下文
      </label>
      <input
        :value="execution.thinkingContextWindowTokens ?? 32768"
        :disabled="execution.thinkingContextWindowTokens === null"
        type="number"
        min="16384"
        max="1048576"
        step="1024"
        class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-sm text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] disabled:cursor-not-allowed disabled:opacity-60"
        @input="update({ thinkingContextWindowTokens: numberValue($event) })"
      />
      <p class="pt-2 text-xs text-[var(--text-secondary)]">
        仅原生 Ollama 的开启思考请求使用；默认上下文为 32K
        token。该上下文窗口是动态恢复的硬限制，系统不会在重试时超过它。
      </p>
    </div>

    <div v-if="showVisionCapability">
      <label
        class="mb-2 flex items-center gap-2 text-sm font-medium text-[var(--text-primary)]"
      >
        <input
          :checked="execution.maxImagesPerRequest !== null"
          type="checkbox"
          class="rounded"
          @change="
            setMaxImagesOverride(($event.target as HTMLInputElement).checked)
          "
        />
        自定义图片上限
      </label>
      <input
        :value="execution.maxImagesPerRequest ?? defaults.maxImagesPerTask"
        :disabled="execution.maxImagesPerRequest === null"
        type="number"
        min="1"
        :max="defaults.maxImagesPerTask"
        class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)] disabled:cursor-not-allowed disabled:opacity-60"
        @input="update({ maxImagesPerRequest: numberValue($event) })"
      />
      <p class="pt-2 text-xs text-[var(--text-secondary)]">
        可进一步收紧全局上限
        {{ defaults.maxImagesPerTask }}；关闭覆盖时沿用全局值。
      </p>
    </div>
  </div>

  <label
    v-if="showAdditionalInstructions"
    class="mt-4 block text-sm font-medium text-[var(--text-primary)]"
  >
    {{ instructionLabel }}
    <textarea
      :value="execution.additionalInstructions"
      maxlength="2000"
      rows="3"
      class="mt-2 w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-sm text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
      :placeholder="instructionPlaceholder"
      @input="
        update({
          additionalInstructions: ($event.target as HTMLTextAreaElement).value,
        })
      "
    />
  </label>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type {
  AIConnectionProfile,
  AIExecutionSettings,
  AITaskExecutionSettings,
} from "@/types/api";

const props = withDefaults(
  defineProps<{
    execution: AITaskExecutionSettings;
    profiles: AIConnectionProfile[];
    defaults: AIExecutionSettings;
    instructionLabel: string;
    instructionPlaceholder: string;
    showVisionCapability?: boolean;
    showAdditionalInstructions?: boolean;
    allowJsonSchema?: boolean;
  }>(),
  {
    showVisionCapability: false,
    showAdditionalInstructions: true,
    allowJsonSchema: false,
  },
);

const emit = defineEmits<{
  "update:execution": [execution: AITaskExecutionSettings];
}>();

const hasOutputOverride = computed(
  () =>
    props.execution.outputTokenLimit !== null ||
    props.execution.thinkingOutputTokenLimit !== null,
);

const update = (patch: Partial<AITaskExecutionSettings>) => {
  emit("update:execution", { ...props.execution, ...patch });
};

const setOutputOverride = (enabled: boolean) => {
  update({
    outputTokenLimit: enabled ? props.defaults.outputTokenLimit : null,
    thinkingOutputTokenLimit: enabled
      ? props.defaults.thinkingOutputTokenLimit
      : null,
  });
};

const setTimeoutOverride = (enabled: boolean) => {
  update({
    timeoutSeconds: enabled ? props.defaults.timeoutSeconds : null,
  });
};

const resolvedFirstTokenDefault = computed(() => {
  const pid = props.execution.profileId.trim();
  if (pid && pid !== "auto") {
    const profile = props.profiles.find((p) => p.id === pid);
    if (profile) return profile.connection.firstTokenTimeoutSeconds;
  }
  const firstEnabled = props.profiles.find((p) => p.enabled);
  return firstEnabled
    ? firstEnabled.connection.firstTokenTimeoutSeconds
    : 30;
});

const effectiveTimeout = computed(() =>
  props.execution.timeoutSeconds ?? props.defaults.timeoutSeconds,
);

const setFirstTokenOverride = (enabled: boolean) => {
  update({
    firstTokenTimeoutSeconds: enabled ? resolvedFirstTokenDefault.value : null,
  });
};

const setThinkingContextOverride = (enabled: boolean) => {
  update({ thinkingContextWindowTokens: enabled ? 32768 : null });
};

const setMaxImagesOverride = (enabled: boolean) => {
  update({
    maxImagesPerRequest: enabled
      ? Math.min(4, props.defaults.maxImagesPerTask)
      : null,
  });
};

const numberValue = (event: Event) =>
  Number((event.target as HTMLInputElement).value);
</script>
