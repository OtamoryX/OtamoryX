<template>
  <div class="fixed inset-0 z-50 overflow-y-auto">
    <div class="flex items-end justify-center min-h-screen pt-4 px-4 pb-20 text-center sm:block sm:p-0">
      <!-- 背景遮罩 -->
      <div
        class="fixed inset-0 bg-gray-500 bg-opacity-75 transition-opacity"
        @click="$emit('close')"
      ></div>

      <!-- 对话框 -->
      <div class="inline-block align-bottom bg-white rounded-lg text-left overflow-hidden shadow-xl transform transition-all sm:my-8 sm:align-middle sm:max-w-lg sm:w-full">
        <div class="bg-white px-4 pt-5 pb-4 sm:p-6 sm:pb-4">
          <div class="sm:flex sm:items-start">
            <div class="mx-auto flex-shrink-0 flex items-center justify-center h-12 w-12 rounded-full bg-blue-100 sm:mx-0 sm:h-10 sm:w-10">
              <svg class="h-6 w-6 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
              </svg>
            </div>
            <div class="mt-3 text-center sm:mt-0 sm:ml-4 sm:text-left flex-1">
              <h3 class="text-lg leading-6 font-medium text-gray-900 mb-4">
                创建分类
              </h3>
              
              <form @submit.prevent="handleSubmit" class="space-y-4">
                <!-- 分类类型选择 -->
                <div>
                  <label class="text-sm font-medium text-gray-700 mb-2 block">分类类型</label>
                  <div class="flex space-x-4">
                    <label class="flex items-center">
                      <input
                        v-model="categoryType"
                        type="radio"
                        value="static"
                        class="h-4 w-4 text-blue-600 border-gray-300 focus:ring-blue-500"
                      />
                      <span class="ml-2 text-sm text-gray-700">静态分类</span>
                    </label>
                    <label class="flex items-center">
                      <input
                        v-model="categoryType"
                        type="radio"
                        value="dynamic"
                        class="h-4 w-4 text-blue-600 border-gray-300 focus:ring-blue-500"
                      />
                      <span class="ml-2 text-sm text-gray-700">动态分类</span>
                    </label>
                  </div>
                </div>

                <!-- 分类名称 -->
                <div>
                  <label for="name" class="block text-sm font-medium text-gray-700 mb-2">
                    分类名称 *
                  </label>
                  <input
                    id="name"
                    v-model="form.name"
                    type="text"
                    required
                    class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                    placeholder="输入分类名称"
                  />
                </div>

                <!-- 描述 -->
                <div>
                  <label for="description" class="block text-sm font-medium text-gray-700 mb-2">
                    描述
                  </label>
                  <textarea
                    id="description"
                    v-model="form.description"
                    rows="3"
                    class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                    placeholder="输入分类描述（可选）"
                  ></textarea>
                </div>

                <!-- 动态分类的搜索条件 -->
                <div v-if="categoryType === 'dynamic'" class="space-y-3">
                  <h4 class="text-sm font-medium text-gray-700">搜索条件</h4>
                  
                  <div>
                    <label class="block text-sm text-gray-600 mb-1">标题关键词</label>
                    <input
                      v-model="searchParams.query"
                      type="text"
                      class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                      placeholder="例如：海贼王"
                    />
                  </div>

                  <div class="grid grid-cols-2 gap-3">
                    <div>
                      <label class="block text-sm text-gray-600 mb-1">最小页数</label>
                      <input
                        v-model.number="searchParams.minPages"
                        type="number"
                        class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                        placeholder="0"
                      />
                    </div>
                    <div>
                      <label class="block text-sm text-gray-600 mb-1">最大页数</label>
                      <input
                        v-model.number="searchParams.maxPages"
                        type="number"
                        class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                        placeholder="999"
                      />
                    </div>
                  </div>

                  <div>
                    <label class="block text-sm text-gray-600 mb-1">标签（用逗号分隔）</label>
                    <input
                      v-model="tagsInput"
                      type="text"
                      class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                      placeholder="例如：少年漫画,冒险"
                    />
                  </div>
                </div>
              </form>
            </div>
          </div>
        </div>
        
        <div class="bg-gray-50 px-4 py-3 sm:px-6 sm:flex sm:flex-row-reverse">
          <button
            @click="handleSubmit"
            :disabled="isLoading || !form.name.trim()"
            class="w-full inline-flex justify-center rounded-md border border-transparent shadow-sm px-4 py-2 bg-blue-600 text-base font-medium text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 sm:ml-3 sm:w-auto sm:text-sm disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {{ isLoading ? '创建中...' : '创建' }}
          </button>
          <button
            @click="$emit('close')"
            type="button"
            class="mt-3 w-full inline-flex justify-center rounded-md border border-gray-300 shadow-sm px-4 py-2 bg-white text-base font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 sm:mt-0 sm:ml-3 sm:w-auto sm:text-sm"
          >
            取消
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { createCategory, createDynamicCategory } from '@/utils/api'
import type { CreateCategoryRequest, CreateDynamicCategoryRequest, SearchParams } from '@/types/api'

const emit = defineEmits<{
  close: []
  created: []
}>()

const isLoading = ref(false)
const categoryType = ref<'static' | 'dynamic'>('static')

const form = ref({
  name: '',
  description: ''
})

const searchParams = ref<SearchParams>({
  query: '',
  minPages: undefined,
  maxPages: undefined,
  tags: []
})

const tagsInput = ref('')

// 处理标签输入
const processedSearchParams = computed(() => ({
  ...searchParams.value,
  tags: tagsInput.value ? tagsInput.value.split(',').map(tag => tag.trim()).filter(Boolean) : undefined
}))

const handleSubmit = async () => {
  if (!form.value.name.trim()) return

  isLoading.value = true
  try {
    if (categoryType.value === 'static') {
      const request: CreateCategoryRequest = {
        name: form.value.name.trim(),
        description: form.value.description.trim() || undefined
      }
      await createCategory(request)
    } else {
      const request: CreateDynamicCategoryRequest = {
        name: form.value.name.trim(),
        description: form.value.description.trim() || undefined,
        searchParams: processedSearchParams.value
      }
      await createDynamicCategory(request)
    }
    
    emit('created')
  } catch (error) {
    console.error('Failed to create category:', error)
    // TODO: 显示错误提示
  } finally {
    isLoading.value = false
  }
}
</script>