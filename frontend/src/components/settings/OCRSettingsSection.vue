<template>
  <div class="space-y-6">
    <GlassCard size="md" radius="lg">
      <div class="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h2 class="text-lg font-medium text-[var(--text-primary)]">
            OCR 辅助内容分析
          </h2>
          <p class="mt-1 text-sm text-[var(--text-secondary)]">
            新漫画入库后，系统会在后台分析抽样页面。OCR 将页面文字提供给视觉 AI
            辅助理解；关闭后仍会进行图像分析。
          </p>
        </div>
        <label
          class="flex items-center gap-2 text-sm text-[var(--text-primary)]"
        >
          <input
            v-model="enabled"
            type="checkbox"
            class="rounded"
            :disabled="loading || saving"
          />
          在内容分析中启用 OCR
        </label>
      </div>
      <p class="mt-3 text-xs text-[var(--text-secondary)]">
        OCR
        不会翻译漫画，也不会在阅读器中提供文字识别。启用后只影响尚未完成的内容分析，已完成的档案不会自动重新分析。
      </p>
      <div
        v-if="isDirty && props.showSaveControls"
        class="mt-4 flex flex-wrap items-center justify-between gap-3 rounded-lg border border-amber-400/40 bg-amber-500/5 px-3 py-2.5"
        role="status"
        aria-live="polite"
      >
        <div class="flex items-center gap-2 text-sm text-[var(--text-primary)]">
          <span class="h-2 w-2 rounded-full bg-amber-500" aria-hidden="true" />
          OCR 设置有未保存的更改
        </div>
        <div class="flex shrink-0 items-center gap-2">
          <GlassButton
            variant="ghost"
            size="sm"
            :disabled="saving"
            @click="discardEnabled"
          >
            放弃
          </GlassButton>
          <GlassButton
            variant="primary"
            size="sm"
            class-name="min-w-[5.5rem]"
            :disabled="saving"
            :loading="saving"
            loading-text="保存中..."
            @click="saveEnabled"
          >
            保存
          </GlassButton>
        </div>
      </div>
      <p
        v-if="message"
        class="mt-3 text-sm"
        :class="
          messageIsError ? 'text-red-500' : 'text-[var(--text-secondary)]'
        "
        :role="messageIsError ? 'alert' : 'status'"
        aria-live="polite"
      >
        {{ message }}
      </p>
      <p
        v-if="settings"
        class="mt-4 break-all text-xs text-[var(--text-secondary)]"
      >
        模型目录：{{ settings.cachePath }}
      </p>
    </GlassCard>

    <GlassCard size="md" radius="lg">
      <button
        type="button"
        class="flex w-full items-start justify-between gap-4 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--accent)]"
        :aria-expanded="showModelManager"
        aria-controls="ocr-model-manager"
        @click="showModelManager = !showModelManager"
      >
        <span>
          <span class="block text-lg font-medium text-[var(--text-primary)]">
            识别语言模型
          </span>
          <span class="mt-1 block text-sm text-[var(--text-secondary)]">
            当前模型：{{ activeModelName }} · 点击展开管理已下载模型
          </span>
        </span>
        <ChevronDownIcon
          class="mt-0.5 h-5 w-5 shrink-0 text-[var(--text-secondary)] transition-transform"
          :class="showModelManager ? 'rotate-180' : ''"
          aria-hidden="true"
        />
      </button>
      <div
        v-if="showModelManager"
        id="ocr-model-manager"
        class="mt-4 space-y-3"
      >
        <div
          v-for="model in settings?.models ?? []"
          :key="model.id"
          class="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3"
        >
          <div class="min-w-0">
            <div class="font-medium text-[var(--text-primary)]">
              {{ model.name }}
            </div>
            <div class="mt-1 text-xs text-[var(--text-secondary)]">
              {{ model.language }} · {{ model.version }} ·
              {{ model.downloaded ? "已下载" : "未下载"
              }}<span v-if="model.active"> · 当前模型</span>
            </div>
            <div v-if="model.error" class="mt-1 text-xs text-red-500">
              {{ model.error }}
            </div>
          </div>
          <div class="flex shrink-0 gap-2">
            <GlassButton
              v-if="!model.downloaded"
              size="sm"
              variant="secondary"
              :disabled="loading || model.loading"
              :loading="model.loading"
              loading-text="下载中..."
              @click="download(model.id)"
              >下载</GlassButton
            >
            <GlassButton
              v-else-if="!model.active"
              size="sm"
              variant="secondary"
              :disabled="loading || model.loading"
              :loading="model.loading"
              loading-text="切换中..."
              @click="activate(model.id)"
              >切换</GlassButton
            >
          </div>
        </div>
      </div>
    </GlassCard>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { ChevronDownIcon } from "@heroicons/vue/24/outline";
import GlassButton from "@/components/base/GlassButton.vue";
import GlassCard from "@/components/base/GlassCard.vue";
import {
  activateOcrModel,
  downloadOcrModel,
  getOcrSettings,
  updateOcrSettings,
} from "@/utils/api";
import type { OcrSettings } from "@/types/api";

interface Props {
  /** Hide local save controls when the parent page provides a shared save bar. */
  showSaveControls?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  showSaveControls: true,
});

const settings = ref<OcrSettings | null>(null);
const enabled = ref(false);
const savedEnabled = ref(false);
const loading = ref(false);
const saving = ref(false);
const message = ref<string | null>(null);
const messageIsError = ref(false);
const showModelManager = ref(false);
const isDirty = computed(() => enabled.value !== savedEnabled.value);
const activeModelName = computed(
  () => settings.value?.models.find((model) => model.active)?.name ?? "未选择",
);
const emit = defineEmits<{
  "dirty-change": [dirty: boolean];
}>();
let refreshTimer: ReturnType<typeof setInterval> | undefined;

const load = async (syncForm = true) => {
  try {
    settings.value = await getOcrSettings();
    if (syncForm || !isDirty.value) {
      enabled.value = settings.value.enabled;
      savedEnabled.value = settings.value.enabled;
    }
    return true;
  } catch (error) {
    messageIsError.value = true;
    message.value =
      error instanceof Error ? error.message : "无法读取 OCR 设置";
    return false;
  }
};

const saveEnabled = async (): Promise<boolean> => {
  saving.value = true;
  try {
    if (!settings.value) {
      throw new Error("OCR 设置尚未加载");
    }
    await updateOcrSettings({
      enabled: enabled.value,
      image: settings.value.image,
      failurePolicy: settings.value.failurePolicy,
    });
    const loaded = await load();
    if (!loaded) return false;
    messageIsError.value = false;
    message.value = enabled.value
      ? "OCR 已启用，后续内容分析会将识别文本作为辅助信息。"
      : "OCR 已关闭，后续内容分析将只使用页面图像。";
    return true;
  } catch (error) {
    messageIsError.value = true;
    message.value =
      error instanceof Error ? error.message : "保存 OCR 设置失败";
    return false;
  } finally {
    saving.value = false;
  }
};

const discardEnabled = () => {
  enabled.value = savedEnabled.value;
  messageIsError.value = false;
  message.value = "已放弃未保存的 OCR 设置。";
};

const download = async (modelId: string) => {
  loading.value = true;
  try {
    await downloadOcrModel(modelId);
    messageIsError.value = false;
    message.value = "模型正在后台下载，完成后可切换为当前模型。";
    await load();
  } catch (error) {
    messageIsError.value = true;
    message.value = error instanceof Error ? error.message : "模型下载失败";
  } finally {
    loading.value = false;
  }
};

const activate = async (modelId: string) => {
  loading.value = true;
  try {
    await activateOcrModel(modelId);
    messageIsError.value = false;
    message.value = "模型正在后台切换，完成后会显示为当前模型。";
    await load();
  } catch (error) {
    messageIsError.value = true;
    message.value = error instanceof Error ? error.message : "模型切换失败";
  } finally {
    loading.value = false;
  }
};

watch(isDirty, (dirty) => emit("dirty-change", dirty), { immediate: true });

defineExpose({ save: saveEnabled, discard: discardEnabled });

onMounted(async () => {
  await load();
  refreshTimer = setInterval(() => void load(false), 3000);
});
onBeforeUnmount(() => {
  if (refreshTimer) clearInterval(refreshTimer);
});
</script>
