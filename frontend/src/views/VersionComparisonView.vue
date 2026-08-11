<template>
  <main class="fixed inset-0 z-50 flex flex-col bg-[var(--bg-primary)] text-[var(--text-primary)]">
    <header class="flex min-h-14 items-center gap-3 border-b border-[var(--border)] px-3 sm:px-5">
      <button class="p-2 text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] hover:text-[var(--text-primary)]" title="返回多版本" @click="goBack">
        <ArrowLeftIcon class="h-5 w-5" />
      </button>
      <div class="min-w-0 flex-1">
        <h1 class="truncate text-sm font-medium">多版本对比</h1>
        <p class="text-[10px] text-[var(--text-tertiary)]">{{ archives.length }} 个版本 · 同步查看第 {{ currentPage }} 页</p>
      </div>
      <div class="flex shrink-0 items-center gap-1">
        <button class="p-2 text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] disabled:opacity-40" title="上一页" :disabled="currentPage <= 1" @click="changePage(-1)">
          <ChevronLeftIcon class="h-5 w-5" />
        </button>
        <span class="min-w-12 text-center text-xs tabular-nums text-[var(--text-secondary)]">{{ currentPage }} / {{ maxComparablePage }}</span>
        <button class="p-2 text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] disabled:opacity-40" title="下一页" :disabled="currentPage >= maxComparablePage" @click="changePage(1)">
          <ChevronRightIcon class="h-5 w-5" />
        </button>
      </div>
    </header>

    <div v-if="isLoadingArchives" class="flex flex-1 items-center justify-center text-sm text-[var(--text-tertiary)]">正在加载对比内容...</div>
    <div v-else-if="archives.length < 2" class="flex flex-1 items-center justify-center px-6 text-center text-sm text-[var(--text-tertiary)]">请选择至少两个可访问的版本进行对比。</div>
    <section v-else class="grid flex-1 grid-cols-1 overflow-y-auto bg-[var(--border)] sm:grid-cols-2" :class="archives.length > 2 ? 'xl:grid-cols-3' : ''">
      <article v-for="archive in archives" :key="archive.id" class="flex min-h-0 flex-col bg-[var(--bg-primary)]">
        <div class="border-b border-[var(--border)] px-3 py-2">
          <h2 class="truncate text-xs font-medium" :title="archive.title">{{ archive.title }}</h2>
          <p class="mt-0.5 text-[10px] text-[var(--text-tertiary)]">{{ archive.pageCount }} 页 · {{ formatSize(archive.fileSize) }}</p>
        </div>
        <div class="flex min-h-64 flex-1 items-center justify-center bg-black/10 p-2">
          <img v-if="pageUrls[archive.id]" :src="pageUrls[archive.id]" :alt="`${archive.title} 第 ${currentPage} 页`" class="max-h-[calc(100vh-9rem)] max-w-full object-contain" />
          <p v-else-if="pageErrors[archive.id]" class="px-4 text-center text-xs text-red-400">该版本无法加载第 {{ currentPage }} 页</p>
          <p v-else class="text-xs text-[var(--text-tertiary)]">正在加载...</p>
        </div>
      </article>
    </section>
  </main>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ArrowLeftIcon, ChevronLeftIcon, ChevronRightIcon } from '@heroicons/vue/24/outline'
import type { Archive } from '@/types/api'
import { getArchive, getArchivePage } from '@/utils/api'

const route = useRoute()
const router = useRouter()
const selectedIds = computed(() => {
  const value = route.query.ids
  const raw = Array.isArray(value) ? value.join(',') : value || ''
  return [...new Set(raw.split(',').filter(Boolean))].slice(0, 4)
})
const archives = ref<Archive[]>([])
const currentPage = ref(1)
const pageUrls = ref<Record<string, string>>({})
const pageErrors = ref<Record<string, boolean>>({})
const isLoadingArchives = ref(false)
const maxComparablePage = computed(() => archives.value.length ? Math.max(1, Math.min(...archives.value.map(archive => archive.pageCount))) : 1)

const clearPageUrls = () => {
  Object.values(pageUrls.value).forEach(url => URL.revokeObjectURL(url))
  pageUrls.value = {}
}

const loadPages = async () => {
  clearPageUrls()
  pageErrors.value = {}
  const page = currentPage.value
  const entries = await Promise.all(archives.value.map(async archive => {
    try {
      return [archive.id, await getArchivePage(archive.id, page)] as const
    } catch {
      return [archive.id, null] as const
    }
  }))
  if (page !== currentPage.value) {
    entries.forEach(([, url]) => { if (url) URL.revokeObjectURL(url) })
    return
  }
  const urls: Record<string, string> = {}
  const errors: Record<string, boolean> = {}
  entries.forEach(([id, url]) => {
    if (url) urls[id] = url
    else errors[id] = true
  })
  pageUrls.value = urls
  pageErrors.value = errors
}

const loadArchives = async () => {
  isLoadingArchives.value = true
  clearPageUrls()
  currentPage.value = 1
  const ids = selectedIds.value
  const entries = await Promise.all(ids.map(async id => await getArchive(id).catch(() => null)))
  if (ids.join(',') !== selectedIds.value.join(',')) return
  archives.value = entries.filter((archive): archive is Archive => archive !== null)
  isLoadingArchives.value = false
  if (archives.value.length) await loadPages()
}

const changePage = (offset: number) => {
  const next = Math.min(maxComparablePage.value, Math.max(1, currentPage.value + offset))
  if (next === currentPage.value) return
  currentPage.value = next
  void loadPages()
}
const goBack = () => {
  if (window.history.state?.back) router.back()
  else router.replace('/library')
}
const formatSize = (bytes: number) => bytes >= 1024 * 1024 ? `${(bytes / 1024 / 1024).toFixed(1)} MB` : `${Math.ceil(bytes / 1024)} KB`

watch(selectedIds, () => { void loadArchives() }, { immediate: true })
onBeforeUnmount(clearPageUrls)
</script>
