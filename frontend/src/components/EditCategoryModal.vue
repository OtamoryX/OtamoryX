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
            <div class="mx-auto flex-shrink-0 flex items-center justify-center h-12 w-12 rounded-full bg-yellow-100 sm:mx-0 sm:h-10 sm:w-10">
              <svg class="h-6 w-6 text-yellow-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
              </svg>
            </div>
            <div class="mt-3 text-center sm:mt-0 sm:ml-4 sm:text-left flex-1">
              <h3 class="text-lg leading-6 font-medium text-gray-900 mb-4">
                编辑分类
              </h3>
              
              <form @submit.prevent="handleSubmit" class="space-y-4">
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

                <!-- 分类类型显示 -->
                <div class="bg-gray-50 p-3 rounded-lg">
                  <div class="flex items-center space-x-2">
                    <svg v-if="isStatic" class="w-4 h-4 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
                    </svg>
                    <svg v-else class="w-4 h-4 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
                    </svg>
                    <span class="text-sm text-gray-600">
                      {{ isStatic ? '静态分类' : '动态分类' }}
                    </span>
                  </div>
                </div>
              </form>
            </div>
          </div>
        </div>
        
        <div class="bg-gray-50 px-4 py-3 sm:px-6 sm:flex sm:flex-row-reverse">
          <button
            @click="handleSubmit"
            :disabled="isLoading || !form.name?.trim()"
            class="w-full inline-flex justify-center rounded-md border border-transparent shadow-sm px-4 py-2 bg-blue-600 text-base font-medium text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 sm:ml-3 sm:w-auto sm:text-sm disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {{ isLoading ? '保存中...' : '保存' }}
          </button>
          <button
            @click="handleDelete"
            :disabled="isLoading"
            class="mt-3 w-full inline-flex justify-center rounded-md border border-red-300 shadow-sm px-4 py-2 bg-white text-base font-medium text-red-700 hover:bg-red-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-red-500 sm:mt-0 sm:ml-3 sm:w-auto sm:text-sm disabled:opacity-50 disabled:cursor-not-allowed"
          >
            删除
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
import { ref, computed, onMounted } from 'vue'
import { updateCategory, deleteCategory } from '@/utils/api'
import type { Category, DynamicCategory, UpdateCategoryRequest } from '@/types/api'

interface Props {
  category: Category | DynamicCategory
}

const props = defineProps<Props>()

const emit = defineEmits<{
  close: []
  updated: []
}>()

const isLoading = ref(false)

const form = ref({
  name: '',
  description: ''
})

const isStatic = computed(() => {
  return 'isStatic' in props.category ? props.category.isStatic : false
})

const handleSubmit = async () => {
  if (!form.value.name?.trim()) return

  isLoading.value = true
  try {
    const request: UpdateCategoryRequest = {
      name: form.value.name.trim(),
      description: form.value.description?.trim() || undefined
    }
    
    await updateCategory(props.category.id, request)
    emit('updated')
  } catch (error) {
    console.error('Failed to update category:', error)
    // TODO: 显示错误提示
  } finally {
    isLoading.value = false
  }
}

const handleDelete = async () => {
  if (!confirm('确定要删除这个分类吗？此操作不可撤销。')) {
    return
  }

  isLoading.value = true
  try {
    await deleteCategory(props.category.id)
    emit('updated')
  } catch (error) {
    console.error('Failed to delete category:', error)
    // TODO: 显示错误提示
  } finally {
    isLoading.value = false
  }
}

onMounted(() => {
  form.value.name = props.category.name
  form.value.description = props.category.description || ''
})
</script>