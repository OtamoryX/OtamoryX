<template>
  <div class="space-y-6">
    <GlassCard size="md" radius="lg">
      <h2 class="mb-4 text-lg font-medium text-[var(--text-primary)]">漫画库配置</h2>
      <div class="space-y-4">
        <div>
          <label class="mb-2 block text-sm font-medium text-[var(--text-primary)]">漫画库路径</label>
          <div class="flex gap-2">
            <GlassInput v-model="systemSettings.comicsPath" placeholder="/path/to/comics" class="flex-1" />
            <GlassButton variant="secondary" @click="emit('select-comics-path')">浏览</GlassButton>
          </div>
          <p class="mt-1 text-sm text-[var(--text-secondary)]">指定漫画文件所在目录</p>
        </div>

        <div>
          <label class="mb-2 block text-sm font-medium text-[var(--text-primary)]">支持文件格式</label>
          <div class="flex flex-wrap gap-2">
            <span
              v-for="format in systemSettings.supportedFormats"
              :key="format"
              class="rounded-full border border-[var(--accent)]/30 bg-[var(--accent)]/15 px-2 py-1 text-xs text-[var(--accent)]"
            >
              .{{ format }}
            </span>
          </div>
        </div>

        <div>
          <label class="mb-2 block text-sm font-medium text-[var(--text-primary)]">最大文件大小 (MB)</label>
          <input
            v-model.number="systemSettings.maxFileSize"
            type="number"
            min="1"
            max="1000"
            class="w-40 rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
          />
        </div>
      </div>
    </GlassCard>

    <GlassCard size="md" radius="lg">
      <h2 class="mb-4 text-lg font-medium text-[var(--text-primary)]">缓存策略</h2>
      <div class="space-y-4">
        <div>
          <label class="mb-2 block text-sm font-medium text-[var(--text-primary)]">缓存策略</label>
          <select
            v-model="cacheSettings.strategy"
            class="w-52 rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
            @change="emit('cache-strategy-change')"
          >
            <option value="conservative">保守策略</option>
            <option value="balanced">平衡策略</option>
            <option value="aggressive">激进策略</option>
            <option value="custom">自定义</option>
          </select>
          <p class="mt-1 text-sm text-[var(--text-secondary)]">{{ cacheStrategyDescription }}</p>
        </div>

        <div
          v-if="cacheSettings.strategy === 'custom'"
          class="space-y-4 rounded-lg border border-[var(--accent)]/40 bg-[var(--bg-tertiary)] p-4"
        >
          <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
            <div>
              <label class="mb-2 block text-sm text-[var(--text-primary)]">最大内存 (MB)</label>
              <input
                v-model.number="cacheSettings.customConfig.maxMemoryMb"
                type="number"
                min="128"
                max="4096"
                class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-card)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
              />
            </div>
            <div>
              <label class="mb-2 block text-sm text-[var(--text-primary)]">最大缓存档案数</label>
              <input
                v-model.number="cacheSettings.customConfig.maxCachedArchives"
                type="number"
                min="5"
                max="100"
                class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-card)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
              />
            </div>
            <div>
              <label class="mb-2 block text-sm text-[var(--text-primary)]">缓存过期时间 (小时)</label>
              <input
                v-model.number="cacheSettings.customConfig.cacheTtlHours"
                type="number"
                min="1"
                max="168"
                class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-card)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
              />
            </div>
          </div>

          <div>
            <label class="mb-2 block text-sm text-[var(--text-primary)]">预加载前后页数</label>
            <div class="grid grid-cols-2 gap-4 sm:w-72">
              <div>
                <label class="mb-1 block text-xs text-[var(--text-secondary)]">前</label>
                <input
                  v-model.number="cacheSettings.customConfig.preloadPrevPages"
                  type="number"
                  min="0"
                  max="10"
                  class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-card)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
                />
              </div>
              <div>
                <label class="mb-1 block text-xs text-[var(--text-secondary)]">后</label>
                <input
                  v-model.number="cacheSettings.customConfig.preloadNextPages"
                  type="number"
                  min="0"
                  max="10"
                  class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-card)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
                />
              </div>
            </div>
          </div>
        </div>

        <div class="rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-4">
          <div class="mb-2 text-sm font-medium text-[var(--text-primary)]">缓存状态</div>
          <div v-if="cacheStatus" class="grid grid-cols-1 gap-3 text-sm sm:grid-cols-2 lg:grid-cols-4">
            <div>
              <span class="text-[var(--text-secondary)]">当前策略:</span>
              <span class="ml-2 font-medium text-[var(--text-primary)]">{{ cacheStatus.current_strategy }}</span>
            </div>
            <div>
              <span class="text-[var(--text-secondary)]">缓存命中率:</span>
              <span class="ml-2 font-medium text-[var(--text-primary)]">{{ formatHitRate(cacheStatus.stats.hit_rate) }}</span>
            </div>
            <div>
              <span class="text-[var(--text-secondary)]">内存使用:</span>
              <span class="ml-2 font-medium text-[var(--text-primary)]">{{ cacheStatus.stats.memory_usage_mb.toFixed(1) }} MB</span>
            </div>
            <div>
              <span class="text-[var(--text-secondary)]">缓存数量:</span>
              <span class="ml-2 font-medium text-[var(--text-primary)]">{{ cacheStatus.stats.cached_archives }}</span>
            </div>
          </div>
          <div class="mt-3 flex flex-wrap gap-2">
            <GlassButton :disabled="isClearingCache" variant="secondary" size="sm" @click="emit('refresh-cache-status')">刷新状态</GlassButton>
            <GlassButton :disabled="isClearingCache" variant="secondary" size="sm" @click="emit('clear-cache', 'pages')">
              {{ clearingCacheScope === 'pages' ? '清理中...' : '清理阅读缓存' }}
            </GlassButton>
            <GlassButton :disabled="isClearingCache" variant="secondary" size="sm" @click="emit('clear-cache', 'covers')">
              {{ clearingCacheScope === 'covers' ? '清理中...' : '清理封面缓存' }}
            </GlassButton>
            <GlassButton :disabled="isClearingCache" variant="danger" size="sm" @click="emit('clear-cache', 'all')">
              {{ clearingCacheScope === 'all' ? '清理中...' : '清空全部缓存' }}
            </GlassButton>
          </div>
          <p class="mt-2 text-xs text-[var(--text-secondary)]">建议先清理阅读缓存，封面缓存会影响列表封面展示。</p>
        </div>
      </div>
    </GlassCard>

    <GlassCard size="md" radius="lg">
      <h2 class="mb-4 text-lg font-medium text-[var(--text-primary)]">图像缓存配置</h2>
      <div class="space-y-4">
        <div>
          <label class="mb-2 block text-sm font-medium text-[var(--text-primary)]">缓存路径</label>
          <div class="flex gap-2">
            <GlassInput v-model="cacheSettings.cachePath" placeholder="/path/to/cache" class="flex-1" />
            <GlassButton variant="secondary" @click="emit('select-cache-path')">浏览</GlassButton>
          </div>
        </div>

        <div class="grid grid-cols-1 gap-4 sm:grid-cols-3">
          <div>
            <label class="mb-2 block text-sm font-medium text-[var(--text-primary)]">缓存大小 (GB)</label>
            <input
              v-model.number="cacheSettings.maxSize"
              type="number"
              min="0.1"
              max="10"
              step="0.1"
              class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
            />
          </div>

          <div>
            <label class="mb-2 block text-sm font-medium text-[var(--text-primary)]">封面质量</label>
            <div class="flex items-center gap-3">
              <input v-model.number="cacheSettings.quality" type="range" min="1" max="100" class="flex-1" />
              <span class="w-12 text-sm text-[var(--text-primary)]">{{ cacheSettings.quality }}%</span>
            </div>
          </div>

          <div>
            <label class="mb-2 block text-sm font-medium text-[var(--text-primary)]">输出格式</label>
            <select
              v-model="cacheSettings.format"
              class="w-full rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
            >
              <option value="JPEG">JPEG</option>
              <option value="PNG">PNG</option>
              <option value="WebP">WebP</option>
            </select>
          </div>
        </div>
      </div>
    </GlassCard>

    <GlassCard size="md" radius="lg">
      <div class="mb-4 flex items-center justify-between gap-4">
        <h2 class="text-lg font-medium text-[var(--text-primary)]">扫描策略</h2>
        <GlassButton :disabled="systemLoading" variant="primary" size="sm" @click="emit('save-scan')">
          {{ systemLoading ? "保存中..." : "仅保存扫描策略" }}
        </GlassButton>
      </div>

      <div class="space-y-3 rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-4">
        <label class="flex items-center gap-2 text-sm text-[var(--text-primary)]">
          <input v-model="scanSettings.enabled" type="checkbox" class="rounded" />
          启用自动扫描
        </label>
        <label class="flex items-center gap-2 text-sm text-[var(--text-primary)]">
          <input v-model="scanSettings.recursive" type="checkbox" class="rounded" />
          递归扫描子目录
        </label>
        <label class="flex items-center gap-2 text-sm text-[var(--text-primary)]">
          <input v-model="scanSettings.ignoreHidden" type="checkbox" class="rounded" />
          忽略隐藏文件
        </label>
        <label class="flex items-center gap-2 text-sm text-[var(--text-primary)]">
          <input v-model="scanSettings.realtimeMonitoring" type="checkbox" class="rounded" />
          实时文件监控
        </label>
      </div>

      <div class="mt-5 rounded-lg border border-[var(--border)] bg-[var(--bg-tertiary)] p-4">
        <div class="mb-2 text-sm font-medium text-[var(--text-primary)]">手动扫描</div>
        <p class="mb-3 text-sm text-[var(--text-secondary)]">手动触发漫画库扫描，检测新文件并写入数据库。</p>
        <GlassButton :disabled="scanLoading" variant="success" size="sm" @click="emit('manual-scan')">
          {{ scanLoading ? "扫描中..." : "开始扫描" }}
        </GlassButton>

        <div
          v-if="scanResult"
          class="mt-3 rounded-lg border p-3 text-sm"
          :class="scanResult.success ? 'border-green-400/30 bg-green-500/10 text-green-500' : 'border-red-400/30 bg-red-500/10 text-red-500'"
        >
          {{ scanResult.message }}
        </div>
      </div>
    </GlassCard>

    <div class="flex justify-end">
      <GlassButton :disabled="systemLoading" variant="primary" class="px-8 py-3" @click="emit('save-system')">
        {{ systemLoading ? "保存中..." : "保存系统与缓存配置" }}
      </GlassButton>
    </div>
  </div>
</template>

<script setup lang="ts">
import GlassButton from "@/components/base/GlassButton.vue";
import GlassCard from "@/components/base/GlassCard.vue";
import GlassInput from "@/components/base/GlassInput.vue";
import type { ScanSettings, SystemSettings } from "@/types/api";
import type { CacheSettingsForm, CacheStatusResponse } from "@/types/settings";
import type { CacheClearScope } from "@/utils/api";

interface Props {
  systemSettings: SystemSettings;
  cacheSettings: CacheSettingsForm;
  scanSettings: ScanSettings;
  cacheStatus: CacheStatusResponse | null;
  cacheStrategyDescription: string;
  systemLoading: boolean;
  scanLoading: boolean;
  scanResult: { success: boolean; message: string } | null;
  isClearingCache: boolean;
  clearingCacheScope: CacheClearScope | null;
}

defineProps<Props>();

const emit = defineEmits<{
  "select-comics-path": [];
  "select-cache-path": [];
  "save-system": [];
  "save-scan": [];
  "manual-scan": [];
  "refresh-cache-status": [];
  "clear-cache": [scope: CacheClearScope];
  "cache-strategy-change": [];
}>();

const formatHitRate = (hitRate: number | undefined) => {
  if (hitRate === undefined || hitRate === null || Number.isNaN(hitRate)) {
    return "0.0%";
  }
  return `${(hitRate * 100).toFixed(1)}%`;
};
</script>
