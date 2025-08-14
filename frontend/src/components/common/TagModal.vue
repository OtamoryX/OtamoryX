<template>
  <BaseModal
    :show="true"
    title="添加标签"
    width="md"
    :z-index="9999"
    @close="$emit('close')"
  >
    <div class="space-y-4">
      <!-- 标签名称 -->
      <div>
        <label for="tagName" class="block text-sm font-medium text-white mb-2">
          标签名称 <span class="text-red-400">*</span>
        </label>
        <input
          id="tagName"
          v-model="form.name"
          type="text"
          required
          class="w-full px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:ring-2 focus:ring-blue-400 focus:bg-white/15 transition-all"
          placeholder="输入标签名称"
          @keyup.enter="handleSubmit"
        />
      </div>

      <!-- 命名空间 -->
      <div>
        <label for="namespace" class="block text-sm font-medium text-white mb-2">
          命名空间
        </label>
        <input
          id="namespace"
          v-model="form.namespace"
          type="text"
          class="w-full px-4 py-3 bg-white/10 border border-white/20 rounded-lg text-white placeholder-white/50 focus:outline-none focus:ring-2 focus:ring-blue-400 focus:bg-white/15 transition-all"
          placeholder="输入命名空间（可选，默认为 general）"
          @keyup.enter="handleSubmit"
        />
      </div>

      <!-- 预设命名空间快捷按钮 -->
      <div>
        <label class="block text-sm font-medium text-white mb-2">
          常用命名空间
        </label>
        <div class="flex flex-wrap gap-2">
          <button
            v-for="preset in presetNamespaces"
            :key="preset"
            type="button"
            class="px-3 py-1 text-sm bg-white/10 hover:bg-white/20 border border-white/20 rounded-md text-white transition-colors"
            @click="form.namespace = preset"
          >
            {{ preset }}
          </button>
        </div>
      </div>

      <!-- 漫画信息 -->
      <div v-if="archive" class="bg-white/5 p-3 rounded-lg border border-white/10">
        <div class="flex items-center space-x-3">
          <svg
            class="w-5 h-5 text-blue-400 flex-shrink-0"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"
            />
          </svg>
          <div class="flex-1 min-w-0">
            <div class="text-white font-medium truncate">{{ archive.title }}</div>
            <div class="text-white/60 text-sm">{{ archive.pageCount }} 页</div>
          </div>
        </div>
      </div>
    </div>

    <template #footer>
      <div class="flex justify-end space-x-3">
        <button
          type="button"
          class="px-6 py-2 text-white/70 hover:text-white bg-white/10 hover:bg-white/20 rounded-lg transition-all duration-200"
          @click="$emit('close')"
        >
          取消
        </button>
        <button
          :disabled="isLoading || !form.name?.trim()"
          class="px-6 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed flex items-center"
          @click="handleSubmit"
        >
          <svg
            v-if="isLoading"
            class="animate-spin -ml-1 mr-2 h-4 w-4"
            fill="none"
            viewBox="0 0 24 24"
          >
            <circle
              class="opacity-25"
              cx="12"
              cy="12"
              r="10"
              stroke="currentColor"
              stroke-width="4"
            />
            <path
              class="opacity-75"
              fill="currentColor"
              d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
            />
          </svg>
          {{ isLoading ? '添加中...' : '添加标签' }}
        </button>
      </div>
    </template>
  </BaseModal>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import BaseModal from '@/components/base/BaseModal.vue'
import type { Archive } from '@/types/api'

interface Props {
  archive?: Archive
}

const props = defineProps<Props>()

const emit = defineEmits<{
  close: []
  submit: [tagName: string, namespace: string]
}>()

const isLoading = ref(false)
const form = ref({
  name: '',
  namespace: 'general',
})

const presetNamespaces = ['general', 'artist', 'series', 'character', 'genre', 'language', 'publisher']

const handleSubmit = async () => {
  if (!form.value.name?.trim()) return

  isLoading.value = true
  try {
    await emit('submit', form.value.name.trim(), form.value.namespace.trim() || 'general')
  } catch (error) {
    console.error('Tag submission error:', error)
  } finally {
    isLoading.value = false
  }
}

onMounted(() => {
  // 聚焦到标签名称输入框
  const tagNameInput = document.getElementById('tagName') as HTMLInputElement
  if (tagNameInput) {
    setTimeout(() => tagNameInput.focus(), 100)
  }
})
</script>