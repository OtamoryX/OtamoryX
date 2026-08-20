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
        仅原生 Ollama 的开启思考请求使用；默认 32768。
      </p>
    </div>
  </div>

  <label class="mt-4 block text-sm font-medium text-[var(--text-primary)]">
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
  }>(),
  { showVisionCapability: false },
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

const setThinkingContextOverride = (enabled: boolean) => {
  update({ thinkingContextWindowTokens: enabled ? 32768 : null });
};

const numberValue = (event: Event) =>
  Number((event.target as HTMLInputElement).value);
</script>
