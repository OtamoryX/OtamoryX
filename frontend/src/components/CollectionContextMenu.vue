<template>
  <div v-if="show" class="fixed inset-0 z-50" @click.self="emit('close')" @contextmenu.prevent.self="emit('close')">
    <div ref="menuRef" :style="{ left: `${position.x}px`, top: `${position.y}px` }" class="absolute w-[320px] max-w-[calc(100vw-20px)] bg-[var(--bg-card)]/95 backdrop-blur-xl border border-[var(--border)] rounded-xl shadow-2xl py-2">
      <div class="px-4 py-3 border-b border-[var(--border)] bg-[var(--bg-primary)]/55">
        <div class="text-sm font-semibold leading-snug text-[var(--text-primary)] break-words">{{ collection?.displayTitle }}</div>
        <div v-if="collection?.subtitle" class="mt-1 text-xs text-[var(--text-tertiary)] break-words">{{ collection.subtitle }}</div>
        <div v-if="collection" class="mt-1 text-xs text-[var(--text-secondary)]">{{ collection.contentCount }} 个内容 · {{ collection.memberCount }} 个文件</div>
      </div>
      <div class="py-1">
        <button class="w-full px-4 py-2.5 text-left text-sm text-[var(--text-primary)] hover:bg-[var(--bg-tertiary)] transition-colors flex items-center" @click="emit('continue-reading')"><BookOpenIcon class="w-4 h-4 mr-3 text-[var(--accent)]" />继续阅读</button>
        <button class="w-full px-4 py-2.5 text-left text-sm text-[var(--text-primary)] hover:bg-[var(--bg-tertiary)] transition-colors flex items-center" @click="emit('open')"><RectangleStackIcon class="w-4 h-4 mr-3 text-[var(--accent)]" />查看合集</button>
        <button class="w-full px-4 py-2.5 text-left text-sm text-[var(--text-primary)] hover:bg-[var(--bg-tertiary)] transition-colors flex items-center" @click="emit('edit')"><PencilSquareIcon class="w-4 h-4 mr-3 text-[var(--accent)]" />编辑合集信息</button>
        <div class="my-1 border-t border-[var(--border)]" />
        <button class="w-full px-4 py-2.5 text-left text-sm text-[var(--text-primary)] hover:bg-[var(--bg-tertiary)] transition-colors flex items-center" @click="emit('rebuild')"><ArrowPathIcon class="w-4 h-4 mr-3 text-[var(--accent)]" />重新识别全部合集</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { nextTick, ref, watch } from 'vue'
import { ArrowPathIcon, BookOpenIcon, PencilSquareIcon, RectangleStackIcon } from '@heroicons/vue/24/outline'
import type { CollectionSummary } from '@/types/api'

const props = defineProps<{ show: boolean; collection: CollectionSummary | null; position: { x: number; y: number } }>()
const emit = defineEmits<{ close: []; open: []; 'continue-reading': []; edit: []; rebuild: [] }>()
const menuRef = ref<HTMLElement | null>(null)
watch(() => props.show, async (show) => {
  if (!show) return
  await nextTick()
  const menu = menuRef.value
  if (!menu) return
  const bounds = menu.getBoundingClientRect()
  if (bounds.right > window.innerWidth - 8) menu.style.left = `${Math.max(8, window.innerWidth - bounds.width - 8)}px`
  if (bounds.bottom > window.innerHeight - 8) menu.style.top = `${Math.max(8, window.innerHeight - bounds.height - 8)}px`
})
</script>
