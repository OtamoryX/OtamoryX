<template>
  <BaseModal :show="show" title="编辑合集信息" @close="emit('close')">
    <div class="space-y-4">
      <label class="block text-sm text-[var(--text-secondary)]">
        主标题
        <input v-model="title" class="mt-1.5 w-full rounded border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-sm text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)]" />
      </label>
      <label class="block text-sm text-[var(--text-secondary)]">
        副标题
        <input v-model="subtitle" placeholder="可选" class="mt-1.5 w-full rounded border border-[var(--border)] bg-[var(--bg-tertiary)] px-3 py-2 text-sm text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)]" />
      </label>
      <div class="flex justify-end gap-2 pt-2">
        <button class="px-3 py-1.5 text-sm text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)] rounded" @click="emit('close')">取消</button>
        <button class="px-3 py-1.5 text-sm text-white bg-[var(--accent)] hover:opacity-90 rounded disabled:opacity-50" :disabled="!title.trim()" @click="emit('save', title.trim(), subtitle.trim())">保存</button>
      </div>
    </div>
  </BaseModal>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import BaseModal from '@/components/base/BaseModal.vue'
import type { CollectionSummary } from '@/types/api'

const props = defineProps<{ show: boolean; collection: CollectionSummary | null }>()
const emit = defineEmits<{ close: []; save: [title: string, subtitle: string] }>()
const title = ref('')
const subtitle = ref('')
watch(() => props.collection, (collection) => {
  title.value = collection?.displayTitle || ''
  subtitle.value = collection?.subtitle || ''
}, { immediate: true })
</script>
