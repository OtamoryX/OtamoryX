<template>
  <div class="space-y-6">
    <GlassCard size="md" radius="lg">
      <div class="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h2 class="text-lg font-medium text-[var(--text-primary)]">漫画 OCR</h2>
          <p class="mt-1 text-sm text-[var(--text-secondary)]">
            OCR 在后端进程内运行，模型按需下载到数据目录，不使用时不会加载模型。
          </p>
        </div>
        <label class="flex items-center gap-2 text-sm text-[var(--text-primary)]">
          <input v-model="enabled" type="checkbox" class="rounded" :disabled="loading || saving" @change="saveEnabled" />
          启用 OCR
        </label>
      </div>
      <p v-if="message" class="mt-4 text-sm text-[var(--text-secondary)]">{{ message }}</p>
      <p v-if="settings" class="mt-4 break-all text-xs text-[var(--text-secondary)]">模型目录：{{ settings.cachePath }}</p>
    </GlassCard>

    <GlassCard size="md" radius="lg">
      <div class="mb-4">
        <h2 class="text-lg font-medium text-[var(--text-primary)]">模型</h2>
        <p class="mt-1 text-sm text-[var(--text-secondary)]">
          切换成功后，正在排队和后续任务都会使用新模型；切换期间的旧任务会自动重试。
        </p>
      </div>
      <div class="space-y-3">
        <div v-for="model in settings?.models ?? []" :key="model.id" class="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-3">
          <div class="min-w-0">
            <div class="font-medium text-[var(--text-primary)]">{{ model.name }}</div>
            <div class="mt-1 text-xs text-[var(--text-secondary)]">
              {{ model.language }} · {{ model.version }} · {{ model.downloaded ? "已下载" : "未下载" }}<span v-if="model.active"> · 当前模型</span>
            </div>
            <div v-if="model.error" class="mt-1 text-xs text-red-500">{{ model.error }}</div>
          </div>
          <div class="flex shrink-0 gap-2">
            <GlassButton v-if="!model.downloaded" size="sm" variant="secondary" :disabled="loading || model.loading" :loading="model.loading" loading-text="下载中..." @click="download(model.id)">下载</GlassButton>
            <GlassButton v-else-if="!model.active" size="sm" variant="secondary" :disabled="loading || model.loading" :loading="model.loading" loading-text="切换中..." @click="activate(model.id)">切换</GlassButton>
          </div>
        </div>
      </div>
    </GlassCard>
  </div>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import GlassButton from "@/components/base/GlassButton.vue";
import GlassCard from "@/components/base/GlassCard.vue";
import { activateOcrModel, downloadOcrModel, getOcrSettings, updateOcrSettings } from "@/utils/api";
import type { OcrSettings } from "@/types/api";

const settings = ref<OcrSettings | null>(null);
const enabled = ref(false);
const loading = ref(false);
const saving = ref(false);
const message = ref<string | null>(null);
let refreshTimer: ReturnType<typeof setInterval> | undefined;

const load = async () => {
  try {
    settings.value = await getOcrSettings();
    enabled.value = settings.value.enabled;
  } catch (error) {
    message.value = error instanceof Error ? error.message : "无法读取 OCR 设置";
  }
};

const saveEnabled = async () => {
  saving.value = true;
  try {
    await updateOcrSettings(enabled.value);
    message.value = enabled.value ? "OCR 已启用" : "OCR 已关闭";
    await load();
  } catch (error) {
    enabled.value = !enabled.value;
    message.value = error instanceof Error ? error.message : "保存 OCR 设置失败";
  } finally {
    saving.value = false;
  }
};

const download = async (modelId: string) => {
  loading.value = true;
  try {
    await downloadOcrModel(modelId);
    message.value = "模型下载已开始";
    await load();
  } catch (error) {
    message.value = error instanceof Error ? error.message : "模型下载失败";
  } finally {
    loading.value = false;
  }
};

const activate = async (modelId: string) => {
  loading.value = true;
  try {
    await activateOcrModel(modelId);
    message.value = "模型已切换，旧任务会按新模型重试";
    await load();
  } catch (error) {
    message.value = error instanceof Error ? error.message : "模型切换失败";
  } finally {
    loading.value = false;
  }
};

onMounted(async () => {
  await load();
  refreshTimer = setInterval(() => void load(), 3000);
});
onBeforeUnmount(() => { if (refreshTimer) clearInterval(refreshTimer); });
</script>
