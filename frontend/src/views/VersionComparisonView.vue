<template>
  <main class="fixed inset-0 z-50 flex flex-col bg-[var(--bg-primary)] text-[var(--text-primary)]">
    <header class="flex min-h-14 items-center gap-3 border-b border-[var(--border)] px-3 sm:px-5">
      <button class="p-2 text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] hover:text-[var(--text-primary)]" title="返回多版本" @click="goBack">
        <ArrowLeftIcon class="h-5 w-5" />
      </button>
      <div class="min-w-0 flex-1">
        <h1 class="truncate text-sm font-medium">多版本对比</h1>
        <p class="text-[10px] text-[var(--text-tertiary)]">{{ archives.length }} 个版本 · 同步基准第 {{ currentPage }} 页</p>
      </div>
      <div class="flex shrink-0 items-center gap-1">
        <button class="p-2 text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] disabled:opacity-40" title="上一组对齐页面" :disabled="currentPage <= minComparablePage" @click="changePage(-1)">
          <ChevronLeftIcon class="h-5 w-5" />
        </button>
        <span class="min-w-16 text-center text-xs tabular-nums text-[var(--text-secondary)]">{{ currentPage }} / {{ maxComparablePage }}</span>
        <button class="p-2 text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] disabled:opacity-40" title="下一组对齐页面" :disabled="currentPage >= maxComparablePage" @click="changePage(1)">
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
          <div class="mt-1 flex items-center justify-between gap-2">
            <p class="text-[10px] text-[var(--text-tertiary)]">{{ archive.pageCount }} 页 · {{ formatSize(archive.fileSize) }}</p>
            <div class="inline-flex h-6 shrink-0 items-center border border-[var(--border)] text-[10px]">
              <button class="flex h-full w-6 items-center justify-center text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] disabled:opacity-35" :disabled="displayPage(archive) <= 1" :title="`将此版本向前对齐一页（当前第 ${displayPage(archive)} 页）`" @click="changeOffset(archive, -1)">
                <MinusIcon class="h-3 w-3" />
              </button>
              <span class="min-w-16 border-x border-[var(--border)] px-1 text-center tabular-nums text-[var(--text-secondary)]">第 {{ displayPage(archive) }} 页</span>
              <button class="flex h-full w-6 items-center justify-center text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] disabled:opacity-35" :disabled="displayPage(archive) >= archive.pageCount" :title="`将此版本向后对齐一页（当前第 ${displayPage(archive)} 页）`" @click="changeOffset(archive, 1)">
                <PlusIcon class="h-3 w-3" />
              </button>
            </div>
          </div>
        </div>
        <div class="flex min-h-64 flex-1 items-center justify-center bg-black/10 p-2">
          <img v-if="pageUrls[archive.id]" :src="pageUrls[archive.id]" :alt="`${archive.title} 第 ${displayPage(archive)} 页`" class="max-h-[calc(100vh-12rem)] max-w-full object-contain" />
          <p v-else-if="pageErrors[archive.id]" class="px-4 text-center text-xs text-red-400">该版本无法加载第 {{ displayPage(archive) }} 页</p>
          <p v-else class="text-xs text-[var(--text-tertiary)]">正在加载...</p>
        </div>
      </article>
    </section>

    <footer v-if="groupId && groupMemberIds.length > 1 && authStore.isAdmin && archives.length > 1" class="flex flex-wrap items-center justify-between gap-2 border-t border-[var(--border)] bg-[var(--bg-secondary)] px-3 py-2 sm:px-5">
      <label class="flex min-w-0 flex-1 items-center gap-2 text-xs text-[var(--text-secondary)]">
        <span class="shrink-0">保留</span>
        <select v-model="keepArchiveId" class="min-w-0 max-w-sm flex-1 border border-[var(--border)] bg-[var(--bg-primary)] px-2 py-1.5 text-[var(--text-primary)] outline-none focus:border-[var(--accent)]">
          <option v-for="archive in archives" :key="archive.id" :value="archive.id">{{ archive.title }}</option>
        </select>
      </label>
      <button class="inline-flex h-8 items-center gap-1.5 bg-red-500 px-3 text-xs text-white hover:bg-red-400 disabled:opacity-50" :disabled="isCleaning || !keepArchiveId" @click="showCleanupConfirmation = true">
        <TrashIcon class="h-3.5 w-3.5" />{{ isCleaning ? '删除中...' : `删除另外 ${groupMemberIds.length - 1} 本` }}
      </button>
    </footer>

    <ConfirmModal
      :show="showCleanupConfirmation"
      title="确认清理多版本"
      :message="cleanupMessage"
      type="danger"
      confirm-text="删除其他版本"
      @close="showCleanupConfirmation = false"
      @confirm="cleanupComparedVersions"
    />
  </main>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ArrowLeftIcon, ChevronLeftIcon, ChevronRightIcon, MinusIcon, PlusIcon, TrashIcon } from '@heroicons/vue/24/outline'
import { useAuthStore } from '@/stores/auth'
import ConfirmModal from '@/components/common/ConfirmModal.vue'
import type { Archive } from '@/types/api'
import { cleanupVersions, getArchive, getArchivePage } from '@/utils/api'

const route = useRoute()
const router = useRouter()
const authStore = useAuthStore()
const selectedIds = computed(() => {
  const value = route.query.ids
  const raw = Array.isArray(value) ? value.join(',') : value || ''
  return [...new Set(raw.split(',').filter(Boolean))].slice(0, 4)
})
const groupId = computed(() => typeof route.query.group === 'string' ? route.query.group : '')
const groupMemberIds = computed(() => {
  const value = route.query.members
  const raw = Array.isArray(value) ? value.join(',') : value || ''
  return [...new Set(raw.split(',').filter(Boolean))]
})
const archives = ref<Archive[]>([])
const currentPage = ref(1)
const pageOffsets = ref<Record<string, number>>({})
const pageUrls = ref<Record<string, string>>({})
const pageErrors = ref<Record<string, boolean>>({})
const isLoadingArchives = ref(false)
const pageRequestId = ref(0)
const keepArchiveId = ref('')
const isCleaning = ref(false)
const showCleanupConfirmation = ref(false)

const displayPage = (archive: Archive) => currentPage.value + (pageOffsets.value[archive.id] || 0)
const minComparablePage = computed(() => archives.value.length ? Math.max(...archives.value.map(archive => 1 - (pageOffsets.value[archive.id] || 0))) : 1)
const maxComparablePage = computed(() => archives.value.length ? Math.max(minComparablePage.value, Math.min(...archives.value.map(archive => archive.pageCount - (pageOffsets.value[archive.id] || 0)))) : 1)
const cleanupMessage = computed(() => {
  const keeper = archives.value.find(archive => archive.id === keepArchiveId.value)
  return `将保留“${keeper?.title || '选中版本'}”，并永久删除另外 ${Math.max(0, groupMemberIds.value.length - 1)} 本。标签、静态分类和阅读进度会迁移到保留版本。`
})

const clearPageUrls = () => {
  Object.values(pageUrls.value).forEach(url => URL.revokeObjectURL(url))
  pageUrls.value = {}
}

const loadPages = async () => {
  clearPageUrls()
  pageErrors.value = {}
  const requestId = ++pageRequestId.value
  const pages = Object.fromEntries(archives.value.map(archive => [archive.id, displayPage(archive)]))
  const entries = await Promise.all(archives.value.map(async archive => {
    try {
      return [archive.id, await getArchivePage(archive.id, pages[archive.id])] as const
    } catch {
      return [archive.id, null] as const
    }
  }))
  if (requestId !== pageRequestId.value) {
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
  pageOffsets.value = {}
  const ids = selectedIds.value
  const entries = await Promise.all(ids.map(async id => await getArchive(id).catch(() => null)))
  if (ids.join(',') !== selectedIds.value.join(',')) return
  archives.value = entries.filter((archive): archive is Archive => archive !== null)
  keepArchiveId.value = archives.value[0]?.id || ''
  isLoadingArchives.value = false
  if (archives.value.length) await loadPages()
}

const changePage = (offset: number) => {
  const next = Math.min(maxComparablePage.value, Math.max(minComparablePage.value, currentPage.value + offset))
  if (next === currentPage.value) return
  currentPage.value = next
  void loadPages()
}

const changeOffset = (archive: Archive, offset: number) => {
  const currentOffset = pageOffsets.value[archive.id] || 0
  const nextOffset = Math.min(archive.pageCount - currentPage.value, Math.max(1 - currentPage.value, currentOffset + offset))
  if (nextOffset === currentOffset) return
  pageOffsets.value = { ...pageOffsets.value, [archive.id]: nextOffset }
  void loadPages()
}

const handleKeydown = (event: KeyboardEvent) => {
  const target = event.target as HTMLElement | null
  if (target?.matches('input, select, textarea, button')) return
  if (event.key === 'ArrowLeft') {
    event.preventDefault()
    changePage(-1)
  } else if (event.key === 'ArrowRight') {
    event.preventDefault()
    changePage(1)
  }
}

const cleanupComparedVersions = async () => {
  if (!groupId.value || !keepArchiveId.value || isCleaning.value) return
  isCleaning.value = true
  try {
    const deleteArchiveIds = groupMemberIds.value.filter(id => id !== keepArchiveId.value)
    await cleanupVersions(groupId.value, keepArchiveId.value, deleteArchiveIds)
    showCleanupConfirmation.value = false
    router.replace('/library')
  } finally {
    isCleaning.value = false
  }
}

const goBack = () => {
  if (window.history.state?.back) router.back()
  else router.replace('/library')
}
const formatSize = (bytes: number) => bytes >= 1024 * 1024 ? `${(bytes / 1024 / 1024).toFixed(1)} MB` : `${Math.ceil(bytes / 1024)} KB`

watch(selectedIds, () => { void loadArchives() }, { immediate: true })
onMounted(() => document.addEventListener('keydown', handleKeydown))
onBeforeUnmount(() => document.removeEventListener('keydown', handleKeydown))
onBeforeUnmount(clearPageUrls)
</script>
